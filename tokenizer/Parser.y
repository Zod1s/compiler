{
module Parser where
import Data.Char (isAlpha, isSpace, isDigit)
}

%name calc
%tokentype { Token }
%error { parseError }

%token
    let { TokenLet }
    in { TokenIn }
    int { TokenInt $$ }
    var { TokenVar $$ }
    '=' { TokenEq }
    '+' { TokenPlus }
    '-' { TokenMinus }
    '*' { TokenTimes }
    '/' { TokenDiv }
    '(' { TokenOB }
    ')' { TokenCB }
    '>' { TokenGT }
    ">=" { TokenGE }
    '<' { TokenLT }
    "<=" { TokenLE }
    ":=" { TokenAssign }

%right in
%nonassoc '<' '>' ">=" "<=" ":="
%left '+' '-'
%left '*' '/'
%left NEG
%%

Exp :: { Exp }
Exp : let var '=' Exp in Exp { Let $2 $4 $6 }
    | Exp '+' Exp { Plus $1 $3 }
    | Exp '-' Exp { Minus $1 $3 }
    | Exp '*' Exp { Times $1 $3 }
    | Exp '/' Exp { Div $1 $3 }
    | Exp ">=" Exp { Ge $1 $3}
    | Exp '>' Exp { Gt $1 $3}
    | Exp "<=" Exp { Le $1 $3}
    | Exp '<' Exp { Lt $1 $3}
    | var ":=" Exp { Assign $1 $3 }
    | '(' Exp ')' { Brack $2 }
    | '-' Exp %prec NEG { Negate $2 }
    | int { Int $1 }
    | var { Var $1 }

{
parseError :: [Token] -> a
parseError _ = error "bo"

data Exp = Let String Exp Exp
         | Plus Exp Exp
         | Minus Exp Exp
         | Times Exp Exp
         | Div Exp Exp
         | Gt Exp Exp
         | Ge Exp Exp
         | Lt Exp Exp
         | Le Exp Exp
         | Brack Exp
         | Negate Exp
         | Int Int
         | Var String
         | Assign String Exp
         deriving (Show)

data Token = TokenLet
           | TokenIn
           | TokenInt Int
           | TokenVar String
           | TokenEq
           | TokenPlus
           | TokenMinus
           | TokenTimes
           | TokenDiv
           | TokenOB
           | TokenCB
           | TokenGT
           | TokenGE
           | TokenLT
           | TokenLE
           | TokenAssign
           deriving Show

lexer :: String -> [Token]
lexer [] = []
lexer (c:cs)
    | isSpace c = lexer cs
    | isAlpha c = lexVar (c:cs)
    | isDigit c = lexNum (c:cs)
lexer ('=':cs) = TokenEq : lexer cs
lexer (':':'=':cs) = TokenAssign : lexer cs
lexer ('+':cs) = TokenPlus : lexer cs
lexer ('-':cs) = TokenMinus : lexer cs
lexer ('*':cs) = TokenTimes : lexer cs
lexer ('/':cs) = TokenDiv : lexer cs
lexer ('(':cs) = TokenOB : lexer cs
lexer (')':cs) = TokenCB : lexer cs
lexer ('>':'=':cs) = TokenGE : lexer cs
lexer ('>':cs) = TokenGT : lexer cs
lexer ('<':'=':cs) = TokenLE : lexer cs
lexer ('<':cs) = TokenLT : lexer cs

lexNum cs = TokenInt (read num) : lexer rest
    where (num,rest) = span isDigit cs

lexVar cs = case span isAlpha cs of
    ("let",rest) -> TokenLet : lexer rest
    ("in",rest)  -> TokenIn : lexer rest
    (var,rest)   -> TokenVar var : lexer rest

parse :: IO()
parse = getLine >>= print . calc . lexer
}
