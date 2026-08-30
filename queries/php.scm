; PHP — definitions, calls, imports/includes, and Laravel route extraction (best-effort)

; Function definitions
(function_definition
  name: (name) @definition.function)

; Class definitions
(class_declaration
  name: (name) @definition.class)

; Method definitions
(method_declaration
  name: (name) @definition.method)

; Calls (simple function calls)
(function_call_expression
  function: (name) @call)

; Includes / requires
(include_expression
  (string) @import.source)

; Laravel routes (basic): Route::get('/x', 'Ctrl@method') or Route::post(...)
; Note: tree-sitter-php may represent the scope as (name) or (qualified_name).

(scoped_call_expression
  scope: (name) @_route
  name: (name) @route.method
  (#eq? @_route "Route")
  (#match? @route.method "^(get|post|put|delete|patch|options|head|any)$")
  arguments: (arguments
    (argument (string) @route.path)
    (argument (string) @route.handler)))

(scoped_call_expression
  scope: (qualified_name) @_route
  name: (name) @route.method
  (#match? @_route "^\\\\?Route$")
  (#match? @route.method "^(get|post|put|delete|patch|options|head|any)$")
  arguments: (arguments
    (argument (string) @route.path)
    (argument (string) @route.handler)))
