> TODO explain how each is a function call etc

### Keys

```jsonc
{ 
	"a": 78,
	"b": 26
}
```

```
[Slice("a")] -> Number(78)
[Slice("b")] -> Number(26)
```

### More values

```json
{ "a": 2, "b": [], "c": 5, "d": {}, "e": "", "f": true, "g": 4, "h": "x", "i": "this is a string" }
```

```
[Slice("a")] -> Number(2)
[Slice("b")] -> EmptyArray
[Slice("c")] -> Number(5)
[Slice("d")] -> EmptyObject
[Slice("e")] -> String("")
[Slice("f")] -> Boolean(true)
[Slice("g")] -> Number(4)
[Slice("h")] -> String("x")
[Slice("i")] -> String("this is a string")
```

### String escapes

```json
{
	"a": "b\n"
}
```

```
[Slice("a")] -> String("b\n")
```

```json
{
	"a": "b\\"
}
```

```
[Slice("a")] -> String("b\\")
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
[Slice("a")] -> Number(-6)
[Slice("b")] -> Number(6)
[Slice("c")] -> Number(60000000000)
```

### Nested

```json
{ 
	"a": {
		"b": 2
	}
}
```

```
[Slice("a"), Slice("b")] -> Number(2)
```

### Arrays

```json
{ 
	"a": [
		3,
		6,
		7
	]
}
```

```
[Slice("a"), Index(0)] -> Number(3)
[Slice("a"), Index(1)] -> Number(6)
[Slice("a"), Index(2)] -> Number(7)
```

### Top level array

```json
[{"x":2}]
```

```
[Index(0), Slice("x")] -> Number(2)
```

### Empty objects

> This works 

```json
{"x":{}}
```

```
[Slice("x")] -> EmptyObject
```

### Empty arrays

```json
{
	"x": []
}
```

```
[Slice("x")] -> EmptyArray
```


```json
{
	"x": [[]],
	"y": [[[]]],
	"z": [[{}]]
}
```

```
[Slice("x"), Index(0)] -> EmptyArray
[Slice("y"), Index(0), Index(0)] -> EmptyArray
[Slice("z"), Index(0), Index(0)] -> EmptyObject
```

### Bad syntax

Each should error

#### Trailing commas

> see configuration for allowing trailing commas

```json
{ "a": 6, }
```

```javascript
[Slice("a")] -> Number(6)
error at "}", reason ExpectedKey
```

```json
[1, ]
```

```javascript
[Index(0)] -> Number(1)
error at "", reason ExpectedEndOfValue
```

### Bad numbers

```json
{ "a": 6NOT }
```

```javascript
[Slice("a")] -> Number(6)
error at "NOT }", reason ExpectedEndOfValue
```

### Mismatched delimiters

```json
{ } }
```

```javascript
[] -> EmptyObject
completed before end of source " }"
```

```json
{ ]
```

```javascript
error at "]", reason ExpectedKey
```

```json
[ }
```

```javascript
error at "}", reason ExpectedValue
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
[Index(0), Slice("x")] -> Number(2)
[Index(1), Slice("y")] -> Number(3)
[Index(1), Slice("y2")] -> Number(5)
[Index(2), Slice("z")] -> Number(4)
```

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
[Index(0), Slice("x")] -> Number(2)
[Index(1), Slice("y")] -> Number(3)
[Index(1), Slice("y2")] -> Number(5)
[Index(2), Slice("z")] -> Number(4)
```

#### Trailing commas

```jsonc
trailing-commas
---
{ "x": 2, "a": [1, 2, ], }
```

```
[Slice("x")] -> Number(2)
[Slice("a"), Index(0)] -> Number(1)
[Slice("a"), Index(1)] -> Number(2)
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
[Slice("x")] -> Number(2)
[] -> Comment(" something")
[Slice("y")] -> Number(3)
```

#### Comments (hash variant)

```jsonc
with-comments
---
{ 
	"x": 2,
	# something
	"y": 3
}
```

```
[Slice("x")] -> Number(2)
[] -> Comment(" something")
[Slice("y")] -> Number(3)
```

#### Comments (multiline variant)

```jsonc
with-comments
---
{ 
	"x": 2, /*
here
on another line
*/
	"y": 3
}
```

```
[Slice("x")] -> Number(2)
[] -> Comment("\nhere\non another line\n")
[Slice("y")] -> Number(3)
```

#### Extra characters

```json
{
	"a": 2
} + 2
```

```
[Slice("a")] -> Number(2)
completed before end of source " + 2"
```

```json
5 + 2
```

```
[] -> Number(5)
completed before end of source " + 2"
```


#### Early return

```json
{"a": "EARLY RETURN"}
```

```javascript
completed before end of source "}" with "returned early"
```

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
[Slice("y")] -> Number(3)
```
