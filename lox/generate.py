import subprocess


def main():
    expr_args_list = [
        "Binary   - left: Box<Expr>, operator: Token, right: Box<Expr>",
        "Grouping - expression: Box<Expr>",
        "Literal  - value: Literal",
        "Unary    - operator: Token, right: Box<Expr>",
        "Variable - name: Token",
        "Assign   - name: Token, value: Box<Expr>",
        "Logical  - left: Box<Expr>, operator: Token, right: Box<Expr>",
        "Call     - callee: Box<Expr>, paren: Token, arguments: Vec<Expr>",
        "Null",
    ]

    stmt_args_list = [
        "Expression - expr: Expr",
        "Print      - expr: Expr",
        "Var        - name: Token, initializer: Box<Expr>",
        "Block      - statements: Vec<Stmt>",
        "IfStmt     - condition: Expr, then_branch: Box<Stmt>, else_branch: Box<Stmt>",
        "WhileStmt  - condition: Expr, body: Box<Stmt>",
        "Null",
    ]

    with open("./src/expr.rs", "w") as file:
        buildAst(
            "Expr",
            "Visitor",
            "#[derive(Clone, Debug, PartialEq)]",
            "use crate::token::{Token, Literal};\n\n",
            expr_args_list,
        )
        buildAst(
            "Stmt",
            "Visitor",
            "#[derive(Clone, Debug, PartialEq)]",
            "use crate::expr::Expr;\nuse crate::token::Token;\n\n",
            stmt_args_list,
        )


def buildAst(enumName, traitName, pragmas, imports, args_list):
    with open(f"./src/{enumName.lower()}.rs", "w") as file:
        file.write(imports)

        enum(file, enumName, pragmas, args_list)
        trait(file, traitName, enumName, args_list)
        impl(file, enumName, traitName, args_list)
    subprocess.run(["rustfmt", f"./src/{enumName.lower()}.rs"])


def enum(file, enumName, pragmas, args_list):
    file.write(f"{pragmas}")
    file.write(f"pub enum {enumName} {{\n")

    for arg in args_list:
        if arg != "Null":
            temp = arg.split(" - ")
            variantName = temp[0].strip()
            variantFields = temp[1].strip().replace(", ", ",\n")
            file.write(f"{variantName} {{\n")
            file.write(variantFields)
            file.write("},\n")
        else:
            file.write("Null,\n")

    file.write("}\n\n")


def trait(file, traitName, enumName, args_list):
    file.write(f"pub trait {traitName}<T> {{\n")
    for arg in args_list:
        if arg != "Null":
            variantName = arg.split(" - ")[0].strip().lower()
            file.write(
                f"fn visit_{variantName}_{enumName.lower()}(&mut self, {enumName.lower()}: &{enumName}) -> T;\n"
            )
    file.write("}\n\n")


def impl(file, enumName, traitName, args_list):
    file.write(f"impl {enumName} {{\n")

    # accept fn
    file.write(
        f"pub fn accept<T>(&self, {traitName.lower()}: &mut impl {traitName}<T>) -> T {{\n"
    )
    file.write("match self {\n")
    for arg in args_list:
        variantName = arg.split(" - ")[0].strip()
        if variantName == "Null":
            file.write(
                f'{enumName}::Null => panic!("calling visit on {enumName}::Null")'
            )
        else:
            file.write(
                f"{enumName}::{variantName} {{ .. }} => {traitName.lower()}.visit_{variantName.lower()}_{enumName.lower()}(self),\n"
            )
    file.write("}\n}\n")

    # constructors
    for arg in args_list:
        if arg != "Null":
            temp = arg.split(" - ")
            variantName = temp[0].strip()
            variantArgs = temp[1].strip()
            variantFields = ",\n".join(
                map(lambda x: x.split(": ")[0], variantArgs.split(", "))
            )
            file.write(
                f"#[inline]\npub fn {variantName.lower()}({variantArgs}) -> {enumName} {{\n"
            )
            file.write(f"{enumName}::{variantName} {{\n")
            file.write(f"{variantFields}")
            file.write("}\n}\n")

    file.write("}\n\n")


if __name__ == "__main__":
    main()
