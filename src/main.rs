#![allow(dead_code)] 
#![allow(unused_variables)] 

// TODO: type{} syntax

use std::collections::HashMap;
use hvm::parser as parser;

fn main() -> Result<(), String> {

  let args: Vec<String> = std::env::args().collect();

  if args.len() <= 2 || args[1] != "check" {
    println!("{:?}", args);
    println!("Usage: kind2 check file.kind");
    return Ok(());
  }

  let path = &args[2];
  let file = match std::fs::read_to_string(path) {
    Ok(code) => read_file(&code)?,
    Err(msg) => read_file(&DEMO_CODE)?,
  };
  let code = compile_file(&file);

  let mut checker = (&CHECKER_HVM[0 .. CHECKER_HVM.find("////INJECT////").unwrap()]).to_string(); 
  checker.push_str(&code);

  //std::fs::write("check.tmp.hvm", checker.clone()).ok(); writes checker to the checker.hvm file

  let mut rt = hvm::Runtime::from_code(&checker)?;
  let main = rt.alloc_code("Main")?;
  rt.normalize(main);
  println!("{}", readback_string(&rt, main)); // TODO: optimize by deserializing term into text directly

  return Ok(());
}

const CHECKER_HVM: &str = include_str!("checker.hvm");

#[derive(Clone, Debug)]
pub enum Term {
  Typ,
  Var { name: String },
  Let { name: String, expr: Box<Term>, body: Box<Term> },
  All { name: String, tipo: Box<Term>, body: Box<Term> },
  Lam { name: String, body: Box<Term> },
  App { func: Box<Term>, argm: Box<Term> },
  Ctr { name: String, args: Vec<Box<Term>> },
  Fun { name: String, args: Vec<Box<Term>> },
}

#[derive(Clone, Debug)]
pub struct Argument {
  eras: bool,
  name: String,
  tipo: Box<Term>,
}

#[derive(Clone, Debug)]
pub struct Rule {
  name: String,
  pats: Vec<Box<Term>>,
  body: Box<Term>,
}

#[derive(Clone, Debug)]
pub struct Entry {
  name: String,
  args: Vec<Box<Argument>>,
  tipo: Box<Term>,
  rules: Vec<Box<Rule>>
}

#[derive(Clone, Debug)]
pub struct File {
  entries: HashMap<String, Box<Entry>>
}

// Parser
// ======

pub fn parse_var(state: parser::State) -> parser::Answer<Option<Box<Term>>> {
  parser::guard(
    Box::new(|state| {
      let (state, head) = parser::get_char(state)?;
      Ok((state, ('a'..='z').contains(&head) || head == '_' || head == '$'))
    }),
    Box::new(|state| {
      let (state, name) = parser::name(state)?;
      Ok((state, Box::new(Term::Var { name })))
    }),
    state,
  )
}

pub fn parse_lam(state: parser::State) -> parser::Answer<Option<Box<Term>>> {
  parser::guard(
    parser::text_parser("@"),
    Box::new(move |state| {
      let (state, _)    = parser::consume("@", state)?;
      let (state, name) = parser::name(state)?;
      let (state, body) = parse_term(state)?;
      Ok((state, Box::new(Term::Lam { name, body })))
    }),
    state,
  )
}

pub fn parse_all(state: parser::State) -> parser::Answer<Option<Box<Term>>> {
  parser::guard(
    Box::new(|state| {
      let (state, all0) = parser::text("(", state)?;
      let (state, name) = parser::name(state)?;
      let (state, all1) = parser::text(":", state)?;
      Ok((state, all0 && all1 && name.len() > 0))
    }),
    Box::new(|state| {
      let (state, _)    = parser::consume("(", state)?;
      let (state, name) = parser::name(state)?;
      let (state, _)    = parser::consume(":", state)?;
      let (state, tipo) = parse_term(state)?;
      let (state, _)    = parser::consume(")", state)?;
      let (state, body) = parse_term(state)?;
      Ok((state, Box::new(Term::All { name, tipo, body })))
    }),
    state,
  )
}

pub fn parse_app(state: parser::State) -> parser::Answer<Option<Box<Term>>> {
  return parser::guard(
    parser::text_parser("("),
    Box::new(|state| {
      parser::list(
        parser::text_parser("("),
        parser::text_parser(""),
        parser::text_parser(")"),
        Box::new(parse_term),
        Box::new(|args| {
          if !args.is_empty() {
            args.into_iter().reduce(|a, b| Box::new(Term::App { func: a, argm: b })).unwrap()
          } else {
            Box::new(Term::Var { name: "?".to_string() })
          }
        }),
        state,
      )
    }),
    state,
  );
}

pub fn parse_let(state: parser::State) -> parser::Answer<Option<Box<Term>>> {
  return parser::guard(
    parser::text_parser("let "),
    Box::new(|state| {
      let (state, _)    = parser::consume("let ", state)?;
      let (state, name) = parser::name1(state)?;
      let (state, _)    = parser::consume("=", state)?;
      let (state, expr) = parse_term(state)?;
      let (state, _)    = parser::text(";", state)?;
      let (state, body) = parse_term(state)?;
      Ok((state, Box::new(Term::Let { name, expr, body })))
    }),
    state,
  );
}

pub fn parse_ctr(state: parser::State) -> parser::Answer<Option<Box<Term>>> {
  parser::guard(
    Box::new(|state| {
      let (state, _)    = parser::text("{", state)?;
      let (state, head) = parser::get_char(state)?;
      Ok((state, ('A'..='Z').contains(&head)))
    }),
    Box::new(|state| {
      let (state, open) = parser::text("{", state)?;
      let (state, name) = parser::name1(state)?;
      if name == "Type" {
        Ok((state, Box::new(Term::Typ)))
      } else {
        let (state, args) = if open {
          parser::until(parser::text_parser("}"), Box::new(parse_term), state)?
        } else {
          (state, Vec::new())
        };
        Ok((state, Box::new(Term::Ctr { name, args })))
      }
    }),
    state,
  )
}

pub fn parse_fun(state: parser::State) -> parser::Answer<Option<Box<Term>>> {
  parser::guard(
    Box::new(|state| {
      let (state, _)    = parser::text("(", state)?;
      let (state, head) = parser::get_char(state)?;
      Ok((state, ('A'..='Z').contains(&head)))
    }),
    Box::new(|state| {
      let (state, open) = parser::text("(", state)?;
      let (state, name) = parser::name1(state)?;
      let (state, args) = if open {
        parser::until(parser::text_parser(")"), Box::new(parse_term), state)?
      } else {
        (state, Vec::new())
      };
      Ok((state, Box::new(Term::Fun { name, args })))
    }),
    state,
  )
}

pub fn parse_term(state: parser::State) -> parser::Answer<Box<Term>> {
  parser::grammar(
    "Term",
    &[
      Box::new(parse_let), // `let `
      Box::new(parse_all), // `(name:`
      Box::new(parse_ctr), // `{Name`
      Box::new(parse_fun), // `(Name`
      Box::new(parse_app), // `(`
      Box::new(parse_lam), // `@`
      Box::new(parse_var), // 
      Box::new(|state| Ok((state, None))),
    ],
    state,
  )
}

pub fn parse_entry(state: parser::State) -> parser::Answer<Box<Entry>> {
  let (state, name) = parser::name1(state)?;
  let (state, args) = parser::until(parser::text_parser(":"), Box::new(parse_argument), state)?;
  let (state, tipo) = parse_term(state)?;
  let (state, head) = parser::peek_char(state)?;
  if head == '=' {
    let (state, _)    = parser::consume("=", state)?;
    let (state, body) = parse_term(state)?;
    let mut pats = vec![];
    for arg in &args {
      pats.push(Box::new(Term::Var { name: arg.name.clone() }));
    }
    let rules = vec![Box::new(Rule { name: name.clone(), pats, body })];
    return Ok((state, Box::new(Entry { name, args, tipo, rules })));
  } else if head == '{' {
    let (state, _)    = parser::consume("{", state)?;
    let name_clone = name.clone();
    let (state, rules) = parser::until(parser::text_parser("}"), Box::new(move |state| parse_rule(state, name_clone.clone())), state)?;
    return Ok((state, Box::new(Entry { name, args, tipo, rules })));
  } else {
    return Ok((state, Box::new(Entry { name, args, tipo, rules: vec![] })));
  }
}

pub fn parse_rule(state: parser::State, name: String) -> parser::Answer<Box<Rule>> {
  let (state, _)    = parser::consume(&name, state)?;
  let (state, pats) = parser::until(parser::text_parser("="), Box::new(parse_term), state)?;
  let (state, body) = parse_term(state)?;
  return Ok((state, Box::new(Rule { name, pats, body })));
}

pub fn parse_argument(state: parser::State) -> parser::Answer<Box<Argument>> {
  let (state, _)    = parser::consume("(", state)?;
  let (state, name) = parser::name1(state)?;
  let (state, _)    = parser::consume(":", state)?;
  let (state, tipo) = parse_term(state)?;
  let (state, _)    = parser::consume(")", state)?;
  return Ok((state, Box::new(Argument { eras: false, name, tipo })));
}

pub fn parse_file(state: parser::State) -> parser::Answer<Box<File>> {
  let (state, entry_vec) = parser::until(Box::new(parser::done), Box::new(parse_entry), state)?;
  let mut entries = HashMap::new();
  for entry in entry_vec {
    entries.insert(entry.name.clone(), entry);
  }
  return Ok((state, Box::new(File { entries })));
}

pub fn show_term(term: &Term) -> String {
  match term {
    Term::Typ => {
      format!("Type")
    }
    Term::Var { name } => {
      format!("{}", name)
    }
    Term::Let { name, expr, body } => {
      let expr = show_term(expr);
      let body = show_term(body);
      format!("let {} = {}; {}", name, expr, body)
    }
    Term::Lam { name, body } => {
      let body = show_term(body);
      format!("@{}({})", name, body)
    }
    Term::App { func, argm } => {
      let mut args = vec![argm];
      let mut expr = func;
      while let Term::App { func, argm } = &**expr {
        args.push(argm);
        expr = func;
      }
      args.reverse();
      format!("({} {})", show_term(expr), args.iter().map(|x| show_term(x)).collect::<Vec<String>>().join(" "))
    }
    Term::All { name, tipo, body } => {
      let body = show_term(body);
      format!("({}: {}) {}", name, show_term(tipo), body)
    }
    Term::Ctr { name, args } => {
      format!("{{{}{}}}", name, args.iter().map(|x| format!(" {}",show_term(x))).collect::<String>())
    }
    Term::Fun { name, args } => {
      format!("({}{})", name, args.iter().map(|x| format!(" {}",show_term(x))).collect::<String>())
    }
  }
}

pub fn show_rule(rule: &Rule) -> String {
  let name = &rule.name;
  let mut pats = vec![];
  for pat in &rule.pats {
    pats.push(show_term(pat));
  }
  let body = show_term(&rule.body);
  format!("{} {} => {}", name, pats.join(" "), body)
}

pub fn show_entry(entry: &Entry) -> String {
  let name = &entry.name;
  let mut args = vec![];
  for arg in &entry.args {
    args.push(format!(" ({}: {})", arg.name, show_term(&arg.tipo)));
  }
  if entry.rules.len() == 0 {
    format!("{}{} : {}", name, args.join(""), show_term(&entry.tipo))
  } else {
    let mut rules = vec![];
    for rule in &entry.rules {
      rules.push(format!("\n  {}", show_rule(rule)));
    }
    format!("{}{} : {} {{{}\n}}", name, args.join(""), show_term(&entry.tipo), rules.join(""))
  }
}

pub fn show_file(file: &File) -> String {
  let mut entries = vec![];
  for entry in file.entries.values() {
    entries.push(show_entry(entry));
  }
  entries.join("\n")
}

pub fn read_term(code: &str) -> Result<Box<Term>, String> {
  parser::read(Box::new(parse_term), code)
}

pub fn read_file(code: &str) -> Result<Box<File>, String> {
  parser::read(Box::new(parse_file), code)
}

// Compiler
// ========

//pub enum Term {
  //Typ,
  //Var { name: String },
  //Let { name: String, expr: Box<Term>, body: Box<Term> },
  //App { func: Box<Term>, argm: Box<Term> },
  //Lam { name: String, body: Box<Term> },
  //All { name: String, tipo: Box<Term>, body: Box<Term> },
  //Ctr { name: String, args: Vec<Box<Term>> },
  //Fun { name: String, args: Vec<Box<Term>> },
//}
pub fn compile_term(term: &Term) -> String { 
  match term {
    Term::Typ => {
      format!("Typ")
    }
    Term::Var { name } => {
      name.clone()
    }
    Term::Let { name, expr, body } => {
      todo!()
    }
    Term::All { name, tipo, body } => {
      format!("(All {} λ{} {})", compile_term(tipo), name, compile_term(body))
    }
    Term::Lam { name, body } => {
      format!("(Lam λ{} {})", name, compile_term(body))
    }
    Term::App { func, argm } => {
      format!("(App {} {})", compile_term(func), compile_term(argm))
    }
    Term::Ctr { name, args } => {
      let mut args_strs : Vec<String> = Vec::new();
      for arg in args {
        args_strs.push(format!(" {}", compile_term(arg)));
      }
      format!("(Ct{} {}.{})", args.len(), name, args_strs.join(""))
    }
    Term::Fun { name, args } => {
      let mut args_strs : Vec<String> = Vec::new();
      for arg in args {
        args_strs.push(format!(" {}", compile_term(arg)));
      }
      format!("(Fn{} {}.{})", args.len(), name, args_strs.join(""))
    }
  }
}

pub fn compile_entry(entry: &Entry) -> String {
  fn compile_type(args: &Vec<Box<Argument>>, tipo: &Box<Term>, index: usize) -> String {
    if index < args.len() {
      let arg = &args[index];
      format!("(All {} λ{} {})", compile_term(&arg.tipo), arg.name, compile_type(args, tipo, index + 1))
    } else {
      compile_term(tipo)
    }
  }

  fn compile_rule(rule: &Rule) -> String {
    let mut pats = vec![];
    for pat in &rule.pats {
      pats.push(format!(" {}", compile_term(pat)));
    }
    let body = compile_term(&rule.body);
    let mut text = String::new();
    //text.push_str(&format!("    (Rule{} {}.{}) = {}\n", rule.pats.len(), rule.name, pats.join(""), body));
    text.push_str(&format!("    (Rule{} {}.{}) = {}\n", rule.pats.len(), rule.name, pats.join(""), body));
    return text;
  }

  fn compile_rule_chk(rule: &Rule, index: usize, vars: &mut u64, args: &mut Vec<String>) -> String {
    if index < rule.pats.len() {
      let (inp_patt_str, var_patt_str) = compile_patt_chk(&rule.pats[index], vars);
      args.push(var_patt_str);
      let head = inp_patt_str;
      let tail = compile_rule_chk(rule, index + 1, vars, args);
      return format!("(LHS {} {})", head, tail);
    } else {
      return format!("(RHS (Rule{} {}.{}))", index, rule.name, args.iter().map(|x| format!(" {}", x)).collect::<Vec<String>>().join(""));
    }
  }

  fn compile_patt_chk(patt: &Term, vars: &mut u64) -> (String, String) {
    match patt {
      Term::Var { .. } => {
        let inp = format!("(Inp {})", vars);
        let var = format!("(Var {})", vars);
        *vars += 1;
        return (inp, var);
      }
      Term::Ctr { name, args } => {
        let mut inp_args_str = String::new();
        let mut var_args_str = String::new();
        for arg in args {
          let (inp_arg_str, var_arg_str) = compile_patt_chk(arg, vars);
          inp_args_str.push_str(&format!(" {}", inp_arg_str));
          var_args_str.push_str(&format!(" {}", var_arg_str));
        }
        let inp_str = format!("(Ct{} {}.{})", args.len(), name, inp_args_str);
        let var_str = format!("(Ct{} {}.{})", args.len(), name, var_args_str);
        return (inp_str, var_str);
      }
      _ => {
        panic!("Invalid left-hand side pattern: {}", show_term(patt));
      }
    }
  }

  let mut result = String::new();
  result.push_str(&format!("    (NameOf {}.) = \"{}\"\n", entry.name, entry.name));
  result.push_str(&format!("    (HashOf {}.) = %{}\n", entry.name, entry.name));
  result.push_str(&format!("    (TypeOf {}.) = {}\n", entry.name, compile_type(&entry.args, &entry.tipo, 0)));
  for rule in &entry.rules {
    result.push_str(&compile_rule(&rule));
  }
  result.push_str(&format!("    (Verify {}.) =\n", entry.name));
  for rule in &entry.rules {
    result.push_str(&format!("      (Cons {}\n", compile_rule_chk(&rule, 0, &mut 0, &mut vec![]))); 
  }
  result.push_str(&format!("      Nil{}\n", ")".repeat(entry.rules.len())));
  return result;
}

pub fn compile_file(file: &File) -> String {
  let mut result = String::new();
  result.push_str(&format!("\n  Functions =\n"));
  result.push_str(&format!("    let fns = Nil\n"));
  for entry in file.entries.values() {
    result.push_str(&format!("    let fns = (Cons {}. fns)\n", entry.    name));
  }
  result.push_str(&format!("    fns\n\n"));
  for entry in file.entries.values() {
    result.push_str(&format!("  // {}\n", entry.name));
    result.push_str(&format!("  // {}\n", "-".repeat(entry.name.len())));
    result.push_str(&format!("\n"));
    result.push_str(&compile_entry(&entry));
    result.push_str(&format!("\n"));
  }
  return result;
}

fn readback_string(rt: &hvm::Runtime, host: u64) -> String {
  let str_cons = rt.get_id("String.cons");
  let str_nil  = rt.get_id("String.nil");
  let mut term = rt.ptr(host);
  let mut text = String::new();
  //let str_cons = rt.
  loop {
    if hvm::get_tag(term) == hvm::CTR {
      let fid = hvm::get_ext(term);
      if fid == str_cons {
        let head = rt.ptr(hvm::get_loc(term, 0));
        let tail = rt.ptr(hvm::get_loc(term, 1));
        if hvm::get_tag(head) == hvm::NUM {
          text.push(std::char::from_u32(hvm::get_num(head) as u32).unwrap_or('?'));
          term = tail;
          continue;
        }
      }
      if fid == str_nil {
        break;
      }
    }
    panic!("Invalid output: {} {}", hvm::get_tag(term), rt.show(host));
  }
  return text;
}

const DEMO_CODE: &str = "
  Bool : Type
    True  : Bool
    False : Bool

  Nat : Type
    Zero             : Nat
    Succ (pred: Nat) : Nat

  List (a: Type) : Type
    Nil  (a: Type)                       : {List a}
    Cons (a: Type) (x: a) (xs: {List a}) : {List a}

  Not (a: Bool) : Bool {
    Not True  = False
    Not False = True
  }

  And (a: Bool) (b: Bool) : Bool {
    And True  True  = True
    And True  False = False
    And False True  = False
    And False False = False
  }

  Negate (xs: {List Bool}) : {List Bool} {
    Negate {Cons Bool x xs} = {Cons Bool (Not x) (Negate xs)}
    Negate {Nil Bool}       = {Nil Bool}
  }

  Tail (a: Type) (xs: {List a}) : {List a} {
    Tail a {Cons t x xs} = xs
  }

  Main (x: Bool) (y: Nat) : {List Bool} = (Tail Bool {Cons Bool x {Cons Bool y {Nil Bool}}})
";
