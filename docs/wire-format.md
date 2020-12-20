# Wire format for shble commands

Shble sends commands as JSON, as a nested array of base64-encoded strings.  JSON to allow for a common format for representing nested arrays that can be trivially understood on the client, and base64 encoding to allow for output encoded non-UTF-8 format.

Multiple messages can make up a single complete whole when sent as a server-side event, they can be broken up event if it makes the JSON unreadable.  Clients should wait for a complete message before decoding the response as JSON, otherwise the parsing will fail or be inaccurate.

Parsing according to user input is done on the server, so each array represents a single row in a line.  If the client's rules don't parse, or if they don't specify a rule, the output will still be a nested array, like

```json
[["<base64-encoded entire output data>"]]
```

This is done to simplify the parsing on the client side.

A philosophy of shble is that even if inputs fail, it tries to give usable output.  In general, fail-fast is the priority for stuff that doesn't deal with user-input; but as a general rule, for functions taking user input:

1. Users should be given feedback on what they did to cause an error
2. The functionality should proceed regularly if the error was "minor" enough (this is vague, but right now "minor" includes cases where filters couldn't be properly parsed, inputs have extra spaces between/before/after words, etc.)

## Example

With an output of

```
total 8
drwxr-xr-x   9 jimberlage  staff   288 Dec 13 10:19 .
drwxr-xr-x+ 91 jimberlage  staff  2912 Dec 19 18:53 ..
drwxr-xr-x  13 jimberlage  staff   416 Dec 16 21:55 .git
drwxr-xr-x   3 jimberlage  staff    96 Dec 13 10:19 .vscode
-rw-r--r--   1 jimberlage  staff     0 Dec 12 14:23 Makefile
-rw-r--r--   1 jimberlage  staff   262 Dec 12 14:22 README.md
drwxr-xr-x   4 jimberlage  staff   128 Dec 19 18:25 docs
drwxr-xr-x  14 jimberlage  staff   448 Dec 13 14:57 gui
drwxr-xr-x   8 jimberlage  staff   256 Dec 13 14:47 server
```

And user input of

- Line separator: `"\\n"`
- Line regex: `"^(?!-rw-r--r--)"`
- Line indices: `"1.."`
- Column separator: `"\\s"`
- Column indices: `"this isn't valid"`

And a message size of 64 bytes (this is unrealistic, just to give an example that will fit in documentation), then the series of messages should be

```
event: stdout
id: 
data: [["ZHJ3eHIteHIteAo=","OQo=","amltYmVybGFnZQo=","c3RhZmYK","Mjg4Cg==","RGVjCg==","MTMK","MTA6MTkK","Lgo="]]


```
