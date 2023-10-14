use crate::types::{Table, Value};
use std::{
    any::{type_name, Any},
    collections::HashMap,
    fmt, hash,
    marker::PhantomData,
    mem,
};

pub trait GcTrace {
    fn format(&self, f: &mut fmt::Formatter, gc: &Gc) -> fmt::Result;
    fn trace(&self, gc: &mut Gc);
    fn size(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct GcObject {
    is_marked: bool,
    size: usize,
    pub object: Box<dyn GcTrace>,
}

pub struct GcRef<T: GcTrace> {
    index: usize,
    _marker: PhantomData<T>,
}

impl<T: GcTrace> Clone for GcRef<T> {
    #[inline]
    fn clone(&self) -> GcRef<T> {
        *self
    }
}

impl<T: GcTrace> Copy for GcRef<T> {}
impl<T: GcTrace> Eq for GcRef<T> {}

impl<T: GcTrace> fmt::Debug for GcRef<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let full_name = type_name::<T>();
        full_name.split("::").last().unwrap();
        write!(f, "ref({}: {})", self.index, full_name)
    }
}

impl<T: GcTrace> PartialEq for GcRef<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl hash::Hash for GcRef<String> {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

pub struct Gc {
    bytes_allocated: usize,
    next_gc: usize,
    grey_stack: Vec<usize>,
    pub objects: Vec<Option<GcObject>>,
    strings: HashMap<String, GcRef<String>>,
    free_slots: Vec<usize>,
}

impl Gc {
    const NEXT_GC: usize = 1024 * 1024;
    const GROW_FACTOR: usize = 2;

    #[inline]
    pub fn new() -> Self {
        Self {
            bytes_allocated: 0,
            next_gc: Self::NEXT_GC,
            grey_stack: Vec::new(),
            objects: Vec::new(),
            strings: HashMap::new(),
            free_slots: Vec::new(),
        }
    }

    #[cfg(not(feature = "debug_gc_stress"))]
    #[inline]
    pub fn should_gc(&self) -> bool {
        self.bytes_allocated > self.next_gc
    }

    #[cfg(feature = "debug_gc_stress")]
    #[inline]
    pub fn should_gc(&self) -> bool {
        true
    }

    pub fn alloc<T: GcTrace + 'static + fmt::Debug>(&mut self, object: T) -> GcRef<T> {
        #[cfg(feature = "debug_gc_log")]
        let repr = format!("{:?}", object);

        let entry = GcObject {
            is_marked: false,
            size: object.size() + mem::size_of::<GcObject>(),
            object: Box::new(object),
        };

        self.bytes_allocated += entry.size;

        let index = match self.free_slots.pop() {
            Some(i) => {
                self.objects[i] = Some(entry);
                i
            }
            None => {
                self.objects.push(Some(entry));
                self.objects.len() - 1
            }
        };

        #[cfg(feature = "debug_gc_log")]
        eprintln!(
            "alloc(id:{}, type:{}: repr: {}, b:{}, t:{})",
            index,
            type_name::<T>(),
            repr,
            self.bytes_allocated,
            self.next_gc,
        );

        GcRef {
            index,
            _marker: PhantomData,
        }
    }

    pub fn intern(&mut self, string: String) -> GcRef<String> {
        if let Some(&string_ref) = self.strings.get(&string) {
            string_ref
        } else {
            let index = self.alloc(string.clone());
            self.strings.insert(string, index);
            index
        }
    }

    pub fn deref<T: GcTrace + 'static>(&self, gcref: GcRef<T>) -> &T {
        self.objects[gcref.index]
            .as_ref()
            .unwrap_or_else(|| panic!("Reference {} already freed", gcref.index))
            .object
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("Reference {} not found", gcref.index))
    }

    pub fn deref_mut<T: GcTrace + 'static>(&mut self, gcref: GcRef<T>) -> &mut T {
        self.objects[gcref.index]
            .as_mut()
            .unwrap_or_else(|| panic!("Reference {} already freed", gcref.index))
            .object
            .as_any_mut()
            .downcast_mut()
            .unwrap_or_else(|| panic!("Reference {} not found", gcref.index))
    }

    fn free(&mut self, index: usize) {
        #[cfg(feature = "debug_gc_log")]
        eprintln!("free (id:{})", index,);
        if let Some(old) = self.objects[index].take() {
            self.bytes_allocated -= old.size;
            self.free_slots.push(index);
        } else {
            panic!("Double free on {}", index);
        }
    }

    pub fn collect_garbage(&mut self) {
        #[cfg(feature = "debug_gc_log")]
        let before = self.bytes_allocated;

        self.trace_references();
        self.remove_white_strings();
        self.sweep();

        self.next_gc = self.bytes_allocated * Self::GROW_FACTOR;

        #[cfg(feature = "debug_gc_log")]
        eprintln!(
            "collected {} bytes (from {} to {}) next at {}",
            before - self.bytes_allocated,
            before,
            self.bytes_allocated,
            self.next_gc
        );
    }

    #[inline]
    pub fn mark_value(&mut self, value: Value) {
        value.trace(self);
    }

    pub fn mark_object<T: GcTrace>(&mut self, object: GcRef<T>) {
        #[cfg(feature = "debug_gc_log")]
        eprintln!("marking {:?}", object);

        if let Some(obj) = self.objects[object.index].as_mut() {
            if obj.is_marked {
                return;
            }

            #[cfg(feature = "debug_gc_log")]
            eprintln!(
                "mark(id:{}, type:{}, val:{:?})",
                object.index,
                type_name::<T>(),
                object
            );

            obj.is_marked = true;
            self.grey_stack.push(object.index);
        } else {
            panic!(
                "Marking an already disposed value at index {}",
                object.index
            );
        }
    }

    #[inline]
    pub fn mark_table(&mut self, table: &Table) {
        for (&k, &v) in table {
            self.mark_object(k);
            self.mark_value(v);
        }
    }

    #[inline]
    fn trace_references(&mut self) {
        while let Some(index) = self.grey_stack.pop() {
            self.blacken_object(index);
        }
    }

    fn blacken_object(&mut self, index: usize) {
        #[cfg(feature = "debug_gc_log")]
        eprintln!("blacken id:{}", index);

        let object = self.objects[index].take();
        object.as_ref().unwrap().object.trace(self);
        self.objects[index] = object;
    }

    fn sweep(&mut self) {
        for i in 0..self.objects.len() {
            if let Some(mut obj) = self.objects[i].as_mut() {
                if obj.is_marked {
                    obj.is_marked = false;
                } else {
                    self.free(i);
                }
            }
        }
    }

    fn remove_white_strings(&mut self) {
        let strings = &mut self.strings;
        let objects = &self.objects;
        strings.retain(|_k, v| objects[v.index].as_ref().unwrap().is_marked);
    }
}

pub struct GcTraceFormatter<'s, T: GcTrace> {
    value: T,
    gc: &'s Gc,
}

impl<'s, T: GcTrace> GcTraceFormatter<'s, T> {
    #[inline]
    pub fn new(value: T, gc: &'s Gc) -> Self {
        GcTraceFormatter { value, gc }
    }
}

impl<'s, T: GcTrace> fmt::Display for GcTraceFormatter<'s, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.format(f, self.gc)
    }
}
