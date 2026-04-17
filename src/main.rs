use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Display,
    fs::File,
    io::Read,
    path::PathBuf,
    rc::Rc,
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
        let result = run(buf, None);
        println!("{result}");
    } else {
        let interactive_scope = Rc::new(RefCell::new(Scope::new()));

        loop {
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;

            let result = run(buf, Some(interactive_scope.clone()));
            println!("{result}");
        }
    };

    Ok(())
}

fn run(source: String, scope: Option<Rc<RefCell<Scope>>>) -> Value {
    let tokens = tokenize(source);
    let ast = parse_program(tokens);
    eval(ast, scope)
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
    Bang,
    Ident(String),
}

fn tokenize(code: String) -> Vec<Token> {
    let mut chars = code.chars().collect::<VecDeque<_>>();
    let mut tokens = vec![];
    let mut current_ident: Option<String> = None;

    let push_ident = |tokens: &mut Vec<Token>, current_ident: &mut Option<String>| {
        if let Some(ident) = current_ident {
            tokens.push(Token::Ident(ident.clone()));
            *current_ident = None;
        }
    };

    while !chars.is_empty() {
        let next = chars.front().unwrap();

        match next {
            'L' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.pop_front();
                tokens.push(Token::Lambda);
            }
            '.' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.pop_front();
                tokens.push(Token::Dot);
            }
            '$' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.pop_front();
                tokens.push(Token::Dollar);
            }
            '=' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.pop_front();
                tokens.push(Token::Equal);
            }
            '(' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.pop_front();
                tokens.push(Token::Open);
            }
            ')' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.pop_front();
                tokens.push(Token::Close);
            }
            ';' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.pop_front();
                tokens.push(Token::Semi);
            }
            '!' => {
                push_ident(&mut tokens, &mut current_ident);
                chars.pop_front();
                tokens.push(Token::Bang);
            }
            '#' => while !matches!(chars.pop_front(), Some('\n')) {},
            other => {
                if other.is_whitespace() {
                    push_ident(&mut tokens, &mut current_ident);
                    chars.pop_front();
                    continue;
                }

                current_ident = Some(current_ident.unwrap_or("".to_string()) + &other.to_string());
                chars.pop_front();
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
    Source(Box<Expr>),
}

fn parse_program(tokens: Vec<Token>) -> Vec<Expr> {
    let mut queue = VecDeque::from(tokens);
    let mut exprs = vec![];

    while !queue.is_empty() {
        exprs.push(parse(&mut queue));
    }

    exprs
}

fn parse(tokens: &mut VecDeque<Token>) -> Expr {
    parse_source(tokens)
}

fn parse_source(tokens: &mut VecDeque<Token>) -> Expr {
    let Token::Bang = tokens.front().unwrap() else {
        return parse_assignment(tokens);
    };

    tokens.pop_front();

    let next = tokens.pop_front().unwrap();

    let Token::Ident(ident) = next else {
        panic!("expected ident after !, found: {next:?}")
    };

    let source = match ident.as_str() {
        "source" => {
            let expr = parse_assignment(tokens);
            Expr::Source(Box::new(expr))
        }
        _ => unreachable!(),
    };

    let next = tokens.pop_front();

    let Some(Token::Semi) = next else {
        panic!("expected ; after sourcing file, found {next:?}");
    };

    source
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
        panic!("expected ident after $, found: {next:?}");
    };

    tokens.pop_front().unwrap();

    let expr = parse_application(tokens);

    let next = tokens.front();

    let Some(Token::Semi) = next else {
        panic!("expected ; after assignment, found: {next:?}");
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
        let next = tokens.front().unwrap();

        if matches!(next, Token::Close | Token::Semi) {
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
        panic!("expected ident after $, found: {next:?}");
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
        panic!("expected . after L, found {next:?}");
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
            let expr = parse(tokens);
            let next = tokens.pop_front().unwrap();
            let Token::Close = next else {
                panic!("expected closing paren, found: {next:?}");
            };

            expr
        }
        Token::Ident(ident) => Expr::Identifier(ident.clone()),
        _ => panic!("expected ident, found: {next:?}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Nothing,
    Name(String),
    Function {
        param: String,
        body: Expr,
        scope: Rc<RefCell<Scope>>,
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

fn eval(ast: Vec<Expr>, scope: Option<Rc<RefCell<Scope>>>) -> Value {
    let root_scope = scope.unwrap_or(Rc::new(RefCell::new(Scope::new())));

    let mut ast = VecDeque::from(ast);
    let mut value = Value::Nothing;

    while !ast.is_empty() {
        value = eval_expr(ast.pop_front().unwrap(), root_scope.clone());
    }

    value
}

fn eval_expr(expr: Expr, scope: Rc<RefCell<Scope>>) -> Value {
    match expr {
        Expr::Assignment { ident, assignment } => {
            scope.borrow_mut().add_var(ident, *assignment);
            Value::Nothing
        }
        Expr::Variable(ident) => {
            let expr = scope.borrow().substitute_var(&ident);
            eval_expr(expr, scope)
        }
        Expr::Application { left, right } => {
            let left = eval_expr(*left, scope.clone());
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

            let function_scope = Rc::new(RefCell::new(inner_scope.borrow().child()));
            function_scope.borrow_mut().add_sub(param, right);

            eval_expr(body, function_scope)
        }
        Expr::Function { param, body } => Value::Function {
            param,
            body: *body,
            scope,
        },
        Expr::Identifier(ident) => {
            let Some(value) = scope.borrow().substitute(&ident) else {
                return Value::Name(ident);
            };

            value
        }
        Expr::Source(expr) => {
            let value = eval_expr(*expr, scope.clone());

            let Value::Name(file_name) = value else {
                panic!("can't source {value}")
            };

            let mut buf = String::new();

            File::open(&file_name)
                .unwrap_or_else(|_| panic!("failed to open file: {file_name}"))
                .read_to_string(&mut buf)
                .unwrap_or_else(|_| panic!("failed to read file: {file_name}"));

            run(buf, Some(scope))
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nothing => write!(f, ""),
            Value::Name(ident) => write!(f, "{ident}"),
            Value::Function { param, body, scope } => {
                let body = eval_expr(body.clone(), scope.clone());
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
            Expr::Source(expr) => {
                write!(f, "!source {expr}")
            }
        }
    }
}
