# Functional Programming & Category Theory Guidelines for Rust

## CRITICAL: Code Style Requirements

### ❌ NEVER Write:
- Classes with methods that mutate self
- Getters/setters
- Builder patterns that mutate state
- Service/Repository/Controller patterns
- Inheritance hierarchies
- Null object patterns
- Factory patterns
- Singleton patterns
- Observer patterns with mutable subscribers

### ✅ ALWAYS Write:
- Pure functions that return new values
- Algebraic Data Types (ADTs) with pattern matching
- Functors, Monads, and Applicatives where appropriate
- Composition over inheritance
- Type-level programming with traits
- Immutable data structures
- Effect systems (Result, Option)
- Morphisms between types
- Natural transformations

## Core Principles

### 1. Data and Functions are Separate
```rust
// BAD - OOP style
impl User {
    pub fn save(&mut self) -> Result<(), Error> {
        self.updated_at = Utc::now();
        database.save(self)
    }
}

// GOOD - FP style
pub fn save_user(user: User) -> Result<User, Error> {
    let updated = User {
        updated_at: Utc::now(),
        ..user
    };
    database.save(updated)
}
```

### 2. Use Type Classes (Traits) for Abstraction
```rust
// Define capabilities, not objects
trait Functor<A> {
    type Wrapped<B>;
    fn fmap<B, F>(self, f: F) -> Self::Wrapped<B>
    where F: FnOnce(A) -> B;
}

trait Monad<A>: Functor<A> {
    fn bind<B, F>(self, f: F) -> Self::Wrapped<B>
    where F: FnOnce(A) -> Self::Wrapped<B>;
    
    fn pure(a: A) -> Self::Wrapped<A>;
}
```

### 3. Compose Functions, Don't Chain Methods
```rust
// BAD - Method chaining
let result = data
    .process()
    .validate()
    .transform()
    .save();

// GOOD - Function composition
let result = pipe!(
    data,
    process,
    validate,
    transform,
    save
);

// Or with explicit composition
let pipeline = compose!(save, transform, validate, process);
let result = pipeline(data);
```

### 4. Use Algebraic Effects
```rust
// Events as data, not side effects
enum Event {
    UserCreated { id: Uuid, name: String },
    UserUpdated { id: Uuid, changes: UserChanges },
}

// Pure function returning effects
fn handle_command(cmd: Command, state: State) -> (State, Vec<Event>) {
    match cmd {
        Command::CreateUser { name } => {
            let id = derive_id(&state, &name);
            let new_state = add_user(state, id, name.clone());
            let events = vec![Event::UserCreated { id, name }];
            (new_state, events)
        }
    }
}
```

### 5. Category Theory Patterns

#### Functors
```rust
impl<A> Functor<A> for Option<A> {
    type Wrapped<B> = Option<B>;
    
    fn fmap<B, F>(self, f: F) -> Option<B> 
    where F: FnOnce(A) -> B {
        self.map(f)
    }
}
```

#### Monoids
```rust
trait Monoid {
    fn mempty() -> Self;
    fn mappend(self, other: Self) -> Self;
}

impl Monoid for String {
    fn mempty() -> Self { String::new() }
    fn mappend(self, other: Self) -> Self { self + &other }
}
```

#### Natural Transformations
```rust
// Transform between functors preserving structure
fn option_to_result<A, E: Default>(opt: Option<A>) -> Result<A, E> {
    opt.ok_or_else(E::default)
}
```

### 6. Avoid Mutable State
```rust
// BAD - Mutable accumulator
let mut sum = 0;
for x in items {
    sum += x;
}

// GOOD - Fold/reduce
let sum = items.iter().fold(0, |acc, x| acc + x);
```

### 7. Use Optics for Nested Updates
```rust
// Instead of mutation, use lenses
let updated_user = lens::address
    .compose(lens::city)
    .set(user, "New York");
```

### 8. Effect Handling
```rust
// Effects as data
enum Effect {
    ReadFile(PathBuf),
    WriteFile(PathBuf, Vec<u8>),
    HttpGet(Url),
}

// Interpreter pattern
async fn interpret(effect: Effect) -> Result<Value, Error> {
    match effect {
        Effect::ReadFile(path) => {
            fs::read(path).await.map(Value::Bytes)
        }
        // ...
    }
}
```

## Type-Level Programming

### Use Phantom Types for Safety
```rust
struct Id<T> {
    value: Uuid,
    _phantom: PhantomData<T>,
}

struct User;
struct Post;

// Type-safe IDs
let user_id: Id<User> = Id::new();
let post_id: Id<Post> = Id::new();
// Can't mix them up!
```

### GATs for Higher-Kinded Types
```rust
trait HKT {
    type Apply<T>;
}

impl HKT for OptionHKT {
    type Apply<T> = Option<T>;
}
```

## Event Sourcing (FP Style)

```rust
// Events are facts
struct EventStore<E> {
    events: Vec<(Cid, E)>,
}

// Pure projection
fn project<E, S>(events: &[E], initial: S, folder: impl Fn(S, &E) -> S) -> S {
    events.iter().fold(initial, folder)
}

// Command handler returns events
fn handle<C, E>(cmd: C) -> Result<Vec<E>, Error> {
    // Pure logic here
}
```

## CRITICAL RULES FOR CLAUDE

1. **NEVER** create "services", "managers", "controllers", or "handlers" as objects
2. **NEVER** use `&mut self` unless absolutely necessary for performance
3. **ALWAYS** prefer returning new values over mutation
4. **ALWAYS** use ADTs and pattern matching over inheritance
5. **NEVER** use design patterns from Gang of Four
6. **ALWAYS** think in terms of transformations, not operations
7. **PREFER** free functions over methods
8. **USE** traits only for type classes and capabilities, not for "interfaces"

## When Asked to Implement Something

1. First, define the data types (ADTs)
2. Then, define pure transformations between them
3. Handle effects at the boundaries
4. Compose functions to build behavior
5. Use type system to enforce invariants