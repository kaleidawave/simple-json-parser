> TODO explain how each is a function call etc

### Keys

```jsonc
{ 
	"a": 78,
	"b": 26
}
```

```
[Slice("a")] -> Number("78")
[Slice("b")] -> Number("26")
```

### More values

> TODO should yield empty objects under option

```json
{ "a": 2, "b": [], "c": 5, "d": {}, "e": "", "f": true, "g": 4, "h": "x", "i": "this is a string" }
```

```
[Slice("a")] -> Number("2")
[Slice("b")] -> EmptyArray
[Slice("c")] -> Number("5")
[Slice("d")] -> EmptyObject
[Slice("e")] -> String("")
[Slice("f")] -> Boolean(true)
[Slice("g")] -> Number("4")
[Slice("h")] -> String("x")
[Slice("i")] -> String("this is a string")
```

### String escapes

> TODO

```json
{
	"a": "b\n"
}
```

```
[Slice("a")] -> String("b\\n")
```

### Numbers

```json
{
	"a": -6,
	"b": 6,
	"c": 6e10
}
```

```
[Slice("a")] -> Number("-6")
[Slice("b")] -> Number("6")
[Slice("c")] -> Number("6e10")
```

### Nested

```jsonc
{ 
	"a": {
		"b": 2
	}
}
```

```
[Slice("a"), Slice("b")] -> Number("2")
```

### Arrays

```jsonc
{ 
	"a": [
		3,
		6,
		7
	]
}
```

```
[Slice("a"), Index(0)] -> Number("3")
[Slice("a"), Index(1)] -> Number("6")
[Slice("a"), Index(2)] -> Number("7")
```

### Bad syntax

Each should error

#### Trailing commas

> see configuration for allowing trailing commas

```json
{ "a": 6, }
```

```
[Slice("a")] -> Number("6")
JSONParseError { at: 10, reason: ExpectedKey }
```

### Bad numbers

```json
{ "a": 6NOT }
```

```
[Slice("a")] -> Number("6")
JSONParseError { at: 8, reason: ExpectedEndOfValue }
```

### Mismatched delimters 1

```json
{ } }
```

```
[] -> EmptyObject
JSONParseError { at: 4, reason: ExpectedEndOfValue }
```

### Mismatched delimters 2

```json
{ ]
```

```
JSONParseError { at: 2, reason: ExpectedKey }
```

### Configuration

#### New line separated

> I get mixed up with separated and delimetered

The following with `top_level_separator = Some("\n")`

```jsonc
new-line-separated
---
{ "x": 2 }
{ "y": 3, "y2": 5 }
{ "z": 4 }
```

> Note each gets an index

```
[Index(0), Slice("x")] -> Number("2")
[Index(1), Slice("y")] -> Number("3")
[Index(1), Slice("y2")] -> Number("5")
[Index(2), Slice("z")] -> Number("4")
```

#### New line separated (2)

The following with `top_level_separator = Some("\n")`

```jsonc
new-line-separated
---
{ 
	"x": 2
}
{ 
	"y": 3, 
	"y2": 5 
}
{ 
	"z": 4
}
```

> Note each gets an index

```
[Index(0), Slice("x")] -> Number("2")
[Index(1), Slice("y")] -> Number("3")
[Index(1), Slice("y2")] -> Number("5")
[Index(2), Slice("z")] -> Number("4")
```

#### Trailing commas

```jsonc
trailing-commas
---
{ "x": 2, "a": [1, 2, ], }
```

```
[Slice("x")] -> Number("2")
[Slice("a"), Index(0)] -> Number("1")
[Slice("a"), Index(1)] -> Number("2")
```

#### Comments

```jsonc
with-comments
---
{ 
	"x": 2,
	// something
	"y": 3
}
```

```
[Slice("x")] -> Number("2")
[Slice("y")] -> Number("3")
```

#### Early

#TODO

#### Partial syntax

```jsonc
partial
---
{ 
	"x": ,
	"y": 3
}
```

```
[Slice("x")] -> Empty
[Slice("y")] -> Number("3")
```
