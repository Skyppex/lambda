use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
    fs::File,
    io::Read,
    path::PathBuf,
};

use clap::Parser;

#[derive(Parser)]
struct Cli {
    file: Option<PathBuf>,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    if let Some(path) = cli.file {
        let mut file = File::open(path)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        run(buf, None)
    } else {
        let mut interactive_scope = Scope::new();

        loop {
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;

            run(buf, Some(&mut interactive_scope))
        }
    };

    Ok(())
}

fn run(source: String, scope: Option<&mut Scope>) {
    let tokens = tokenize(source);
    let ast = parse(tokens);
    let result = eval(ast, scope);
    println!("{result}");
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Lambda,
    Dot,
    Dollar,
    Equal,
    Open,
    Close,
    Semi,
    Ident(String),
}

fn tokenize(code: String) -> Vec<Token> {
    let mut chars = code.chars().collect::<Vec<_>>();
    let mut tokens = vec![];
    let mut current_ident: Option<String> = None;

    let push_ident = |tokens: &mut Vec<Token>, current_ident: &mut Option<String>| {
        if let Some(ident) = current_ident {
            tokens.push(Token::Ident(ident.clone()));
            *current_ident = None;
        }
    };

    while !chars.is_empty() {
        let next = chars.first().unwrap();

        match next {
            'L' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.remove(0);
                tokens.push(Token::Lambda);
            }
            '.' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.remove(0);
                tokens.push(Token::Dot);
            }
            '$' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.remove(0);
                tokens.push(Token::Dollar);
            }
            '=' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.remove(0);
                tokens.push(Token::Equal);
            }
            '(' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.remove(0);
                tokens.push(Token::Open);
            }
            ')' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.remove(0);
                tokens.push(Token::Close);
            }
            ';' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.remove(0);
                tokens.push(Token::Semi);
            }
            other => {
                if other.is_whitespace() {
                    push_ident(&mut tokens, &mut current_ident);
                    chars.remove(0);
                    continue;
                }

                current_ident = Some(current_ident.unwrap_or("".to_string()) + &other.to_string());
                chars.remove(0);
            }
        }
    }

    push_ident(&mut tokens, &mut current_ident);
    tokens
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Expr {
    Assignment {
        ident: String,
        assignment: Box<Expr>,
    },
    Variable(String),
    Application {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Function {
        param: String,
        body: Box<Expr>,
    },
    Identifier(String),
}

fn parse(tokens: Vec<Token>) -> Vec<Expr> {
    let mut queue = VecDeque::from(tokens);
    let mut exprs = vec![];

    while !queue.is_empty() {
        exprs.push(parse_assignment(&mut queue));
    }

    exprs
}

fn parse_assignment(tokens: &mut VecDeque<Token>) -> Expr {
    let Token::Dollar = tokens.front().unwrap() else {
        return parse_application(tokens);
    };

    let mut tokens_iter = tokens.iter();
    tokens_iter.next(); // $
    tokens_iter.next(); // ident
    let third = tokens_iter.next(); // =?

    let Some(Token::Equal) = third else {
        return parse_application(tokens);
    };

    tokens.pop_front();

    let next = tokens.pop_front().unwrap();

    let Token::Ident(ident) = next else {
        panic!("expected ident after $, found: {:?}", next);
    };

    let next = tokens.pop_front().unwrap();

    let Token::Equal = next else {
        return Expr::Variable(ident);
    };

    let expr = parse_application(tokens);

    let next = tokens.front();

    let Some(Token::Semi) = next else {
        panic!("expected ; after assignment, found: {:?}", next);
    };

    tokens.pop_front();

    Expr::Assignment {
        ident,
        assignment: Box::new(expr),
    }
}

fn parse_application(tokens: &mut VecDeque<Token>) -> Expr {
    let mut left = parse_variable(tokens);

    while tokens.front().is_some() {
        let next = tokens.front();

        if matches!(next, None | Some(Token::Close) | Some(Token::Semi)) {
            return left;
        }

        let right = parse_variable(tokens);

        left = Expr::Application {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    left
}

fn parse_variable(tokens: &mut VecDeque<Token>) -> Expr {
    let Token::Dollar = tokens.front().unwrap() else {
        return parse_function(tokens);
    };

    tokens.pop_front();
    let next = tokens.pop_front().unwrap();

    let Token::Ident(ident) = next else {
        panic!("expected ident after $, found: {:?}", next);
    };

    Expr::Variable(ident)
}

fn parse_function(tokens: &mut VecDeque<Token>) -> Expr {
    let Token::Lambda = tokens.front().unwrap() else {
        return parse_primary(tokens);
    };

    tokens.pop_front();
    let param = tokens.pop_front().unwrap();

    let Token::Ident(param) = param else {
        panic!("expected ident as param after L, found {:?}", param);
    };

    let next = tokens.pop_front().unwrap();

    let Token::Dot = next else {
        panic!("expected . after L, found {:?}", next);
    };

    let body = parse_application(tokens);

    Expr::Function {
        param,
        body: Box::new(body),
    }
}

fn parse_primary(tokens: &mut VecDeque<Token>) -> Expr {
    let next = tokens.pop_front().unwrap();

    match next {
        Token::Open => {
            let expr = parse_application(tokens);
            let next = tokens.pop_front().unwrap();
            let Token::Close = next else {
                panic!("expected closing paren, found: {:?}", next);
            };

            expr
        }
        Token::Ident(ident) => Expr::Identifier(ident.clone()),
        _ => panic!("expected ident, found: {:?}", next),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Nothing,
    Name(String),
    Function {
        param: String,
        body: Expr,
        scope: Scope,
    },
    UnresolvedApplication {
        left: Box<Value>,
        right: Box<Value>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Scope {
    subs: HashMap<String, Value>,
    vars: HashMap<String, Expr>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Scope {
            subs: HashMap::new(),
            vars: HashMap::new(),
            parent: None,
        }
    }

    fn child(&self) -> Self {
        Scope {
            subs: HashMap::new(),
            vars: HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    fn add_sub(&mut self, ident: String, value: Value) {
        self.subs.insert(ident, value);
    }

    fn substitute(&self, ident: &str) -> Option<Value> {
        self.subs
            .get(ident)
            .cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.substitute(ident)))
    }

    fn add_var(&mut self, ident: String, value: Expr) {
        self.vars.insert(ident, value);
    }

    fn substitute_var(&self, ident: &str) -> Expr {
        self.vars
            .get(ident)
            .cloned()
            .or_else(|| self.parent.as_ref().map(|p| p.substitute_var(ident)))
            .unwrap_or_else(|| panic!("undefined variable ${ident}"))
    }
}

fn eval(ast: Vec<Expr>, scope: Option<&mut Scope>) -> Value {
    let mut root_scope = Scope::new();
    let root_scope = scope.unwrap_or(&mut root_scope);

    let mut ast = VecDeque::from(ast);
    let mut value = Value::Nothing;

    while !ast.is_empty() {
        value = eval_expr(ast.pop_front().unwrap(), root_scope);
    }

    value
}

fn eval_expr(expr: Expr, scope: &mut Scope) -> Value {
    match expr {
        Expr::Assignment { ident, assignment } => {
            scope.add_var(ident, *assignment);
            Value::Nothing
        }
        Expr::Variable(ident) => {
            let expr = scope.substitute_var(&ident);
            eval_expr(expr, scope)
        }
        Expr::Application { left, right } => {
            let left = eval_expr(*left, scope);
            let right = eval_expr(*right, scope);

            let Value::Function {
                param,
                body,
                scope: inner_scope,
            } = left
            else {
                return Value::UnresolvedApplication {
                    left: Box::new(left),
                    right: Box::new(right),
                };
            };

            let mut function_scope = inner_scope.child();
            function_scope.add_sub(param, right);

            eval_expr(body, &mut function_scope)
        }
        Expr::Function { param, body } => Value::Function {
            param,
            body: *body,
            scope: scope.clone(),
        },
        Expr::Identifier(ident) => {
            let Some(value) = scope.substitute(&ident) else {
                return Value::Name(ident);
            };

            value
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nothing => write!(f, ""),
            Value::Name(ident) => write!(f, "{ident}"),
            Value::Function { param, body, scope } => {
                let body = eval_expr(body.clone(), &mut scope.clone());
                write!(f, "λ{param}.{body}")
            }
            Value::UnresolvedApplication { left, right } => {
                write!(f, "({left} {right})")
            }
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Assignment { .. } => write!(f, ""),
            Expr::Variable(ident) => {
                write!(f, "${ident}")
            }
            Expr::Application { left, right } => {
                write!(f, "({left} {right})")
            }
            Expr::Function { param, body } => {
                write!(f, "λ{param}.{body}")
            }
            Expr::Identifier(ident) => {
                write!(f, "{ident}")
            }
        }
    }
}
