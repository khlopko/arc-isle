# ArcISLE
 -- The **I**nterface **S**pecification **L**anguag**e**.

Its goal is to provide a lightweight way to describe virtually any type of interfaces in the software and provide more advanced tools for the analysis and governance.

# Status 

The project is in early development stage. At the moment, it is capable of parsing a document created according
to the specification described below, and emit errors in places that does not match without 
failing all the parsing.

Nearest plans is to:

- [x] Add types declarations validation. Right now we collect declared types, but does not verify that all usages
stict to that declarations and emit errors.
    - [ ] Still missing precise localisation of missing declaration, good to add at some point before completion.
- [ ] *Command-line interface.* 
- [ ] Check speed on larger documents. So far it has been tested on a really small API specification, large and
more real-world example is required to define if there optimizations to be done.
- [ ] OpenAPI <> ArcISLE convertation.
- [ ] Mix types and routes declaration
- [ ] Different route versions
- [ ] Default response code guess

# Structure

All specification is written using YAML solely. It allows to write less, do this faster, and still have powerful tools to cover wide range of needs. JSON has inconvenient limitations, like requiring quotes almost everywhere, which make it harder to write manually.

Main sections are following:

- `types` — describes list of objects used in an API.
- `interfaces` — describes endpoints provided by an API.

By default, it is recommended to not maintain single document with description of everything, but start decomposing files into two groups: types and interfaces. This could be done by importing subset of the document:

```yaml
types:
    _import: types.yml
```

Imports also can include list of files:

```yaml
_import:
    - file1.yml
    - file2.yml
    - file3.yml
    - file4.yml
```

Path to the file is resolved relatively to the root document, from which an import is requested.

# Types

API usually contains a lot of specific types, like user, or post, or transaction, etc. These entities appear in many places as we start defining interfaces. By defining them upfront and reusing we can simplify workflow in future. That’s what types for.

Types declarations is allowed inside `types` section (or separated files included inside that section). To declare a type, specify its name and set of fields. Each field has its name and type, defined after a semicolon.

```yaml
user:
    id: uuid
    name: str
    role: str
```

You can also nest types up to 3 levels (this limitation is artificial and made for improved readability):

```yaml
settings:
    version: str?
    flags:
        a: bool
        b: bool	
        c: bool
```

Types can be referenced inside other types:

```yaml
post:
    title: str
    body: str
    author: user
```

## Built-in

Specification supports a set of default types: primitives, containers, and formats. They serve to cover basic needs and allow to build custom types.

### Primitives

All data we are working with has some basic type after all, most essential of them are supported:

- `int` - represents integer numbers
- `double` - represents double precision number
- `bool` - represents boolean
- `str` - unicode strings

### Containers

To serve needs of collections or nesting JSON, there are two container type:

- `array[type]` or `array`
- `dict[key, value]` or `dict`

Square braces act as inner element definition, by specifying type inside of them you denote that it is expected, for example, an array of integers, or a dictionary keyed by string and containing an integer as value. There are no limitations of types that can be used.

No-braces notation removes typing of elements which might be useful in certain cases, but does not recommended in general use.

### Specific Formats

Some types of data are frequently used, and specifying them simply as string might means loosing information. To improve that, specification offers some default types, identified as commonly used.

- `timestamp` — acts as double, denotes that this double is expected to be a time in UNIX format.
- `date_iso8601` — acts as string, denotes and validates field to confirm ISO 8601 date standard.
- `uuid` — acts as string, denotes and validates field to contain a valid UUID value.
- `url` —

## Optionality

Fields inside types might be required or optional. To simplify declaration each field has optionality parameter represented by `?` symbol. Adding it in the end of the field type denotes that this field is optional and might by omitted in the instance of this type.

```yaml
title: str
description: str?
connections: array[uuid]?
```

Optionality *only* ****refers to a field, not a type, which means specifying, e.g. `array[int?]` is *not* a valid syntax.

## JSON Schema

All types are compatible with JSON Schema by design. Any implementation of this specification has to provide a way to generate valid schema from types.

# Interfaces

---

Now, heart of every REST API is actually endpoints, which called interfaces here as they are interface between server and client(s). Declaration of an interface a bit more complex, than of a type. Two mandatory fields of each interface is `path` and `method`:

```yaml
- path: news
  method: get
```

Path could be parameterised using curly brackets:

```yaml
- path: users/{user_id}
```

It is recommended to include type name in parameter name if possible to increase readability. Compare following versions:

- `users/{id}/posts/{id}/comments/{id}`
- `users/{user_id}/posts/{post_id}/comments/{comment_id}`

Leaving aside question of nesting decision here, second option is far easier to understand just by looking at it, since we clearly understand what each parameter represents.

Rest of options depend on details request, and follow HTTP standards.

| Field | Required | Purpose | Restrictions | Possible values |
| --- | --- | --- | --- | --- |
| query | No | Specifies query parameters of a request. | Allowed only within GET and HEAD requests. | Required to be a valid custom type. |
| body | No | Specifies body of a request. | Allowed within POST, PUT, and PATCH requests. | Required to be a valid custom type. |
| body_type | No | Specifies type of a body. | Allowed only together with body present. | Supports currently only form-data value. |
| response | No | Specifies response of a request. | No restrictions. | Any type is allowed in response. |

This is enough to specify basic requirements for endpoints with success flow in mind, for example:

```yaml
types:
  news_entry:
    id: str
    title: str
    link: url
interfaces:
    - path: news
      method: get
      query:
        search: str?
      response:
        items: array[news_entry]
        next_page_link: url?
    - path: news
      method: post
      body:
        title: str
        link: url
      response: news_entry
    - path: news/{entry_id}
      method: delete
```

## Different responses

An advanced scenario is when we want to define custom responses for different cases, like success, failure, permissions error, etc. To support it, `response` field supports variable content in the following way:

```yaml
- path: news/{entry_id}
  method: delete
  response:
    200: news_entry
    4xx: 
        code: int
        reason: str?
    5xx:
        message: str
    501:
        code: int
        reason: str?
```

You can specify either concrete status code, like `200` or `501` in example above, or pattern for a family of status codes, like `4xx` and `5xx`, and you can combine both styles.

By default `2xx` family is assumed if response field does not specify any code.

