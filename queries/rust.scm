; Rust — symbol definitions, calls, and imports

; Function definitions
(function_item
  name: (identifier) @definition.function)

; Struct definitions
(struct_item
  name: (type_identifier) @definition.struct)

; Enum definitions
(enum_item
  name: (type_identifier) @definition.enum)

; Trait definitions
(trait_item
  name: (type_identifier) @definition.interface)

; Impl blocks — capture the type being implemented
(impl_item
  type: (type_identifier) @definition.type)

; Type aliases
(type_item
  name: (type_identifier) @definition.type)

; Const items
(const_item
  name: (identifier) @definition.variable)

; Static items
(static_item
  name: (identifier) @definition.variable)

; Let bindings (inside functions — captured for completeness)
(let_declaration
  pattern: (identifier) @definition.variable)

; Function calls
(call_expression
  function: (identifier) @call)

; Method calls
(call_expression
  function: (field_expression
    field: (field_identifier) @call))

; Scoped calls (e.g. Module::function())
(call_expression
  function: (scoped_identifier
    name: (identifier) @call))

; Macro invocations
(macro_invocation
  macro: (identifier) @call)

; Use declarations
(use_declaration
  argument: (scoped_identifier
    name: (identifier) @import.name))

(use_declaration
  argument: (use_as_clause
    path: (scoped_identifier
      name: (identifier) @import.name)))
