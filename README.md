# Kind

A minimal, efficient and practical proof and programming language. Under the hoods, it is basically Haskell, except purer and with dependent types. On the surface, it looks more like TypeScript. Compared to most proof assistants, Kind has:

1. The smallest core. Check this [700-LOC](https://github.com/moonad/FormCoreJS/blob/master/FormCore.js) reference implementation.

2. More powerful type-level features. Check [this article](https://github.com/uwu-tech/Kind/blob/master/blog/1-beyond-inductive-datatypes.md) on super-inductive datatypes.

3. A collection of friendly syntax sugars that make it feel less scary. Check [SYNTAX.md](https://github.com/uwu-tech/Kind/blob/master/SYNTAX.md).

4. A self-hosted implementation. Check it [here](https://github.com/uwu-tech/Kind/tree/master/base/Kind)!

5. Efficient real-world compilers. Check [App.kind](https://github.com/uwu-tech/Kind/blob/master/base/App.kind), [MiniMMO.kind](https://github.com/uwu-tech/Kind/blob/master/base/App/MiniMMO.kind), and [http://uwu.tech/](http://uwu.tech) for many apps. 

    *Currently disabled for a major update that will bring rollback netcode for all apps! Check on May 17!*

Usage
-----
![npm](https://img.shields.io/npm/v/kind-lang)  

```bash
npm i -g kind-lang                             # installs Kind
git clone https://github.com/uwu-tech/Kind     # clones base libs
cd Kind/base                                   # enters base libs
kind Main                                      # checks Main.kind
kind Main --run                                # runs Main
```

*Right now, you must be at `kind/base` to use the language.*

Haskell and Scheme compilers and releases expected around June.

Examples
--------

### A 'Hello, world!'

```javascript
Main: IO(Unit)
  IO {
    IO.print("Hello, world!")
  }
```

### Some algorithms

```javascript
// Quicksort
quicksort(list: List<Nat>): List<Nat>
  case list {
    nil:
      []
    cons:
      fst = list.head
      min = filter!((x) x <? list.head, list.tail)
      max = filter!((x) x >? list.head, list.tail)
      quicksort(min) ++ [fst] ++ quicksort(max)
  }
```

```javascript
// Sum (using recursion)
sum(list: List(Nat)): Nat
  case list {
    nil  : 0
    cons : list.head + sum(list.tail)
  }
```

```javascript
// List (using fold)
sum(list: List(Nat)): Nat
  List.fold!(list)!(0, Nat.add)
```

```javascript
// List (using loop)
sum(list: List(Nat)): Nat
  sum = 0
  for x in list with sum:
    x + sum
  sum
```

```c
// Some Map, Maybe, String and Nat syntax sugars
maps: Nat
  key  = "toe"
  map  = {"tic": 1, "tac": 2, key: 3} // Map.from_list!([{"tic",1}, ...])
  map  = map{"tic"} <- 100            // Map.set!("tic", 100, map)
  map  = map{"tac"} <- 200            // Map.set!("tac", 200, map)
  map  = map{ key } <- 300            // Map.set!(key, 300, map)
  val0 = map{"tic"} <> 0              // Maybe.default!(Map.get!("tic",map), 0)
  val1 = map{"tac"} <> 0              // Maybe.default!(Map.get!("tac",map), 0)
  val2 = map{ key } <> 0              // Maybe.default!(Map.get!(key, map), 0)
  val0 + val1 + val2                  // Nat.add(val0, Nat.add(val1, val2))
```

Check many List algorithms on [base/List](https://github.com/uwu-tech/Kind/tree/master/base/List)!

### Some types

```javascript
// A struct
type User {
  new(name: String, birth: Date, avatar: Image)
}

// A simple pair
type Pair <A: Type, B: Type> {
  new(fst: A, snd: B)
}

// A dependent pair
type Sigma <A: Type, B: A -> Type> {
  new(fst: A, snd: B(fst))
}

// A list
type List <A: Type> {
  nil
  cons(head: A, tail: List(A))
}

// A list with a statically known size
type Vector <A: Type> ~ (size: Nat) {
  nil                                              ~ (size = 0) 
  cons(size: Nat, head: Nat, tail: Vector(A,size)) ~ (size = 1 + size)
}

// The propositional equality
type Equal <A: Type, a: A> ~ (b: A) {
  refl ~ (b = a)
}
```

Check all core types on [base](https://github.com/uwu-tech/Kind/tree/master/base)!

### Some proofs

```javascript
// Proof that `a == a + 0`
Nat.add.zero(a: Nat): a == Nat.add(a, 0)
  case a {
    zero: refl
    succ: apply(Nat.succ, Nat.add.zero(a.pred))
  }!

// Proof that `1 + (a + b) == a + (1 + b)`
Nat.add.succ(a: Nat, b: Nat): Nat.succ(a + b) == (a + Nat.succ(b))
  case a {
    zero: refl
    succ: apply(Nat.succ, Nat.add.succ(a.pred, b))
  }!

// Proof that addition is commutative
Nat.add.comm(a: Nat, b: Nat): (a + b) == (b + a)
  case a {
    zero:
      Nat.add.zero(b)
    succ: 
      p0 = Nat.add.succ(b, a.pred)
      p1 = Nat.add.comm(b, a.pred)
      p0 :: rewrite X in Nat.succ(X) == _ with p1
  }!
```

Check some Nat proofs on [base/Nat/add](https://github.com/uwu-tech/Kind/tree/master/base/Nat/add)!

### A web app

```javascript
// Render function
App.Hello.draw: App.Draw<App.Hello.State>
  (state)
  DOM.node("div", {}, {"border": "1px solid black"}, [
    DOM.node("div", {}, {"font-weight": "bold"}, [DOM.text("Hello, world!")])
    DOM.node("div", {}, {}, [DOM.text("Clicks: " | Nat.show(state@local))])
    DOM.node("div", {}, {}, [DOM.text("Visits: " | Nat.show(state@global))])
  ])

// Event handler
App.Hello.when: App.When<App.Hello.State>
  (event, state)
  case event {
    init: IO {
      App.watch!(App.room_zero)
      App.new_post!(App.room_zero, App.empty_post)
    }
    mouse_down: IO {
      App.set_local!(state@local + 1)
    }
  } default App.pass!
```

Source: [base/App/Hello.kind](https://github.com/uwu-tech/Kind/blob/master/base/App/Hello.kind)

Output: [App.Hello.js](https://github.com/uwu-tech/Kind/blob/master/web/src/apps/App.Hello.js)

Live Demo: [http://uwu.tech/App.Hello](http://uwu.tech/App.Hello)

You can create your own uwu-tech app by adding a file to `base/App`!

Resources
---------

- Syntax reference: [SYNTAX.md](SYNTAX.md).

- Theorem proving tutorial: [THEOREMS.md](THEOREMS.md).

- Base library: [base](https://github.com/uwu-tech/Kind/tree/master/base).

- [Telegram chat](https://t.me/formality_lang)! 

- Discord server (TODO)

[trusted core]: https://github.com/moonad/FormCoreJS

[FormCore-to-Haskell]: https://github.com/moonad/FormCoreJS/blob/master/FmcToHs.js

[kind.js]: https://github.com/uwu-tech/Kind/blob/master/bin/js/base/kind.js

[Agda]: https://github.com/agda/agda

[Idris]: https://github.com/idris-lang/Idris-dev

[Coq]: https://github.com/coq/coq

[Lean]: https://github.com/leanprover/lean

[Absal]: https://medium.com/@maiavictor/solving-the-mystery-behind-abstract-algorithms-magical-optimizations-144225164b07

[JavaScript compiler]:https://github.com/moonad/FormCoreJS/blob/master/FmcToJs.js
