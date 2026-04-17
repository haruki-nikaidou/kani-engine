//! Rhai scripting engine wrapper.
//!
//! Exposes `f`, `sf`, `tf`, and `mp` (as `rhai::Map`) into the script scope
//! so KAG scripts can read/write game variables with idiomatic Rhai code.
//!
//! This replaces the original JavaScript `eval` / `evalScript` approach.

use rhai::{Dynamic, Engine, EvalAltResult, Map, Scope};

use crate::error::InterpreterError;

// ─── Engine wrapper ───────────────────────────────────────────────────────────

/// A Rhai scripting engine with persistent scope variable bindings.
///
/// The four KAG variable scopes are stored as Rhai `Map` objects:
/// - `f`  — per-play flags
/// - `sf` — persistent system flags
/// - `tf` — transient flags
/// - `mp` — current macro parameters
pub struct ScriptEngine {
    engine: Engine,
    scope: Scope<'static>,
}

impl std::fmt::Debug for ScriptEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptEngine").finish()
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptEngine {
    /// Create a new engine with all four KAG variable maps initialised to
    /// empty `Map`s and pushed into scope.
    pub fn new() -> Self {
        let engine = Engine::new();
        let mut scope = Scope::new();

        scope.push("f", Map::new());
        scope.push("sf", Map::new());
        scope.push("tf", Map::new());
        scope.push("mp", Map::new());

        Self { engine, scope }
    }

    // ── Variable access ───────────────────────────────────────────────────────

    /// Return a clone of the `f` (game flags) map.
    pub fn f(&self) -> Map {
        self.scope.get_value::<Map>("f").unwrap_or_default()
    }

    /// Return a clone of the `sf` (system flags) map.
    pub fn sf(&self) -> Map {
        self.scope.get_value::<Map>("sf").unwrap_or_default()
    }

    /// Return a clone of the `tf` (transient flags) map.
    pub fn tf(&self) -> Map {
        self.scope.get_value::<Map>("tf").unwrap_or_default()
    }

    /// Return a clone of the `mp` (macro params) map.
    pub fn mp(&self) -> Map {
        self.scope.get_value::<Map>("mp").unwrap_or_default()
    }

    /// Replace the entire `mp` map (called at macro entry).
    pub fn set_mp(&mut self, mp: Map) {
        set_map_in_scope(&mut self.scope, "mp", mp);
    }

    /// Read a single value from the `f` map by key.
    pub fn get_f(&self, key: &str) -> Option<Dynamic> {
        self.f().get(key).cloned()
    }

    /// Write a single value into the `f` map.
    pub fn set_f(&mut self, key: impl Into<String>, value: Dynamic) {
        let mut map = self.f();
        map.insert(key.into().into(), value);
        set_map_in_scope(&mut self.scope, "f", map);
    }

    /// Write a single value into the `sf` map.
    pub fn set_sf(&mut self, key: impl Into<String>, value: Dynamic) {
        let mut map = self.sf();
        map.insert(key.into().into(), value);
        set_map_in_scope(&mut self.scope, "sf", map);
    }

    /// Write a single value into the `tf` map.
    pub fn set_tf(&mut self, key: impl Into<String>, value: Dynamic) {
        let mut map = self.tf();
        map.insert(key.into().into(), value);
        set_map_in_scope(&mut self.scope, "tf", map);
    }

    // ── Variable clearing ─────────────────────────────────────────────────────

    /// Clear all `f` (game flags) variables.
    pub fn clear_f(&mut self) {
        set_map_in_scope(&mut self.scope, "f", Map::new());
    }

    /// Clear all `sf` (system flags) variables.
    pub fn clear_sf(&mut self) {
        set_map_in_scope(&mut self.scope, "sf", Map::new());
    }

    /// Clear all `tf` (transient flags) variables.
    pub fn clear_tf(&mut self) {
        set_map_in_scope(&mut self.scope, "tf", Map::new());
    }

    /// Remove a single key from a named variable scope (`"f"`, `"sf"`, or `"tf"`).
    ///
    /// Silently does nothing if the key or scope does not exist.
    pub fn remove_key(&mut self, scope_name: &str, key: &str) {
        let mut map = match scope_name {
            "f" => self.f(),
            "sf" => self.sf(),
            "tf" => self.tf(),
            _ => return,
        };
        map.remove(key);
        set_map_in_scope(&mut self.scope, scope_name, map);
    }

    // ── Evaluation API ────────────────────────────────────────────────────────

    /// Evaluate an arbitrary Rhai expression or statement block.
    ///
    /// Variables mutated inside `script` are automatically reflected back into
    /// the persistent scope because Rhai's `eval_with_scope` shares the scope
    /// reference.
    pub fn exec(&mut self, script: &str) -> Result<Dynamic, String> {
        
        self
            .engine
            .eval_with_scope::<Dynamic>(&mut self.scope, script)
            .map_err(|e| rhai_error_msg(&e))
    }

    /// Evaluate an expression and coerce the result to `bool`.
    ///
    /// Used for `cond=` parameters and `[if exp=…]` conditions.
    pub fn eval_bool(&mut self, expr: &str) -> Result<bool, String> {
        // Wrap in parentheses to handle bare expressions
        let wrapped = format!("({})", expr);
        let result = self
            .engine
            .eval_with_scope::<Dynamic>(&mut self.scope, &wrapped)
            .map_err(|e| rhai_error_msg(&e))?;

        // Flexible truthiness: bools, integers (0 = false), strings ("" = false)
        let b = if result.is_bool() {
            result.as_bool().unwrap_or(false)
        } else if result.is_int() {
            result.as_int().unwrap_or(0) != 0
        } else if result.is::<String>() {
            !result.into_string().unwrap_or_default().is_empty()
        } else {
            !result.is_unit()
        };

        Ok(b)
    }

    /// Evaluate an expression and convert the result to a display string.
    ///
    /// Used for `[emb exp=…]`.
    pub fn eval_to_string(&mut self, expr: &str) -> Result<String, String> {
        let wrapped = format!("({})", expr);
        let result = self
            .engine
            .eval_with_scope::<Dynamic>(&mut self.scope, &wrapped)
            .map_err(|e| rhai_error_msg(&e))?;
        Ok(result.to_string())
    }

    /// Evaluate an expression, returning `Dynamic::UNIT` on any error rather
    /// than propagating.  Mirrors the original `embScript` behaviour which
    /// swallows evaluation errors.
    pub fn eval_soft(&mut self, expr: &str) -> Dynamic {
        let wrapped = format!("({})", expr);
        self.engine
            .eval_with_scope::<Dynamic>(&mut self.scope, &wrapped)
            .unwrap_or(Dynamic::UNIT)
    }

    // ── Snapshot helpers ──────────────────────────────────────────────────────

    /// Serialise a named variable map (`"f"`, `"sf"`, `"tf"`, or `"mp"`) to a
    /// `serde_json::Value`.
    ///
    /// Returns `InterpreterError::SerializationError` on serialisation failure.
    pub fn map_to_json(&self, name: &str) -> Result<serde_json::Value, InterpreterError> {
        let map = match name {
            "f" => self.f(),
            "sf" => self.sf(),
            "tf" => self.tf(),
            "mp" => self.mp(),
            _ => Map::new(),
        };
        serde_json::to_value(&map).map_err(|e| InterpreterError::SerializationError(e.to_string()))
    }

    /// Deserialise a `serde_json::Value` back into a named scope map.
    /// Existing scope entries are replaced.
    pub fn restore_map(
        &mut self,
        name: &str,
        json: &serde_json::Value,
    ) -> Result<(), InterpreterError> {
        let map: Map = serde_json::from_value(json.clone())
            .map_err(|e| InterpreterError::SerializationError(e.to_string()))?;
        set_map_in_scope(&mut self.scope, name, map);
        Ok(())
    }

    /// Resolve a parameter value string that may start with `&` (entity
    /// expression) or `%key|default` (macro param reference).
    ///
    /// Returns the concrete string value to be used at runtime.
    pub fn resolve_entity(&mut self, expr: &str) -> String {
        let result = self.eval_soft(expr);
        if result.is_unit() {
            String::new()
        } else {
            result.to_string()
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn set_map_in_scope(scope: &mut Scope<'_>, name: &str, map: Map) {
    if let Some(val) = scope.get_mut(name) {
        *val = Dynamic::from(map);
    } else {
        scope.push(name.to_owned(), map);
    }
}

fn rhai_error_msg(e: &EvalAltResult) -> String {
    e.to_string()
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_bool_true() {
        let mut eng = ScriptEngine::new();
        assert!(eng.eval_bool("true").unwrap());
        assert!(eng.eval_bool("1 + 1 == 2").unwrap());
    }

    #[test]
    fn test_eval_bool_false() {
        let mut eng = ScriptEngine::new();
        assert!(!eng.eval_bool("false").unwrap());
        assert!(!eng.eval_bool("1 == 2").unwrap());
    }

    #[test]
    fn test_eval_to_string() {
        let mut eng = ScriptEngine::new();
        let s = eng.eval_to_string("40 + 2").unwrap();
        assert_eq!(s, "42");
    }

    #[test]
    fn test_exec_sets_variable() {
        let mut eng = ScriptEngine::new();
        let _ = eng.exec("let counter = 10;").unwrap();
        let val = eng.eval_to_string("counter").unwrap();
        assert_eq!(val, "10");
    }

    #[test]
    fn test_f_map_persistence() {
        let mut eng = ScriptEngine::new();
        let _ = eng.exec(r#"f.visited = true;"#).unwrap();
        let map = eng.f();
        assert_eq!(
            map.get("visited").and_then(|v| v.as_bool().ok()),
            Some(true)
        );
    }

    #[test]
    fn test_set_f_and_read_back() {
        let mut eng = ScriptEngine::new();
        eng.set_f("score", Dynamic::from(42_i64));
        let val = eng.get_f("score");
        assert!(val.is_some());
        assert_eq!(val.unwrap().as_int().unwrap(), 42);
    }

    #[test]
    fn test_sf_map() {
        let mut eng = ScriptEngine::new();
        eng.set_sf("unlocked", Dynamic::from(true));
        let map = eng.sf();
        assert_eq!(
            map.get("unlocked").and_then(|v| v.as_bool().ok()),
            Some(true)
        );
    }

    #[test]
    fn test_eval_soft_swallows_error() {
        let mut eng = ScriptEngine::new();
        let result = eng.eval_soft("this_does_not_exist_xyz");
        // Should not panic; returns unit or fallback
        assert!(result.is_unit() || result.is::<String>());
    }

    #[test]
    fn test_macro_mp_params() {
        let mut eng = ScriptEngine::new();
        let mut mp = Map::new();
        mp.insert("greeting".into(), Dynamic::from("Hello".to_string()));
        eng.set_mp(mp);
        let val = eng.eval_to_string("mp.greeting").unwrap();
        assert_eq!(val, "Hello");
    }

    #[test]
    fn test_cross_eval_variable_visibility() {
        let mut eng = ScriptEngine::new();
        let _ = eng.exec("let x = 5;").unwrap();
        let result = eng.eval_bool("x > 3").unwrap();
        assert!(result);
    }
}
