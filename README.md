## Funkcjonalność

Język silnie i statycznie typowany. Wszystkie zmienne są mutowalne. Argumenty do funkcji mogą być przekazywane przez wartość albo przez referencję. Z funkcji można zwrócić tylko wartość albo nic.

1. Funkcje:
   - Język obsługuje definicje funkcji.
   - Funkcje mogą przyjmować parametry i zwracać wartości określonego typu.
2. Obsługiwane typy danych:
   - i64 (liczby całkowite), f64 (liczby zmiennoprzecinkowe), str (łańcuchy znaków), bool (prawda / fałsz).
   - void (brak zwracanej wartości z funkcji)
3. Instrukcje Warunkowe:
   - Instrukcje warunkowe if wraz z opcjonalnym else.
4. Pętle:
   - Pętla for, w której iterator nie musi być definiowany ani aktualizowany.
5. Instrukcje Kontroli Przepływu:
   - Instrukcja break są używane do przerwania wykonania pętli lub bloku instrukcji.
6. Instrukcje Wyboru:
   - Instrukcje switch pozwalają na wielokrotne rozgałęzianie programu w zależności od różnych warunków.
   - Wykonywany jest każdy blok instrukcji pod spełnionym warunkiem, chyba że w którymś poprzednim był break
7. Rekurencja: Język wspiera rekurencję.
8. Operacje na Zmiennych:
   - Dostępne są operacje przypisania (=), operatory arytmetyczne (+, -, \*, /), porównania (==, <, <=, >, >=, !=), oraz operatory logiczne (np. !=).
9. Operacje na Tekście:
   - Wprowadzenie tekstu, konkatenacja i wyświetlenie go na konsoli.
10. Castowanie:
    - i64 i f64 mogą być castowane na siebie nawzajem, na stringa i na boolean (jeżeli są <= 0 to będzie false inaczej prawda)
    - string może być castowany na i64 i na f64, ze zgłoszeniem błędów, oraz na boolean (pusty string oznacza false inaczej true)
11. Funkcje wbudowane:
    - print(...): wypisuje argumenty na standardowe wyjście wraz ze znakiem końca linii

## Gramatyka

### Część składniowa

**program** = { function_declaration | assign_or_call | if_statement | for_statement | switch_statement | declaration, ";" };

**comment** = "#" , {unicode_character - "\n"}, "\n";

```
# some 1 line comment
```

**function_declaration** = “fn”, identifier, "(", parameters, ")", “:”, type | “void”, statement_block;

```
fn do_sth(&i64 x, i64 y=0): void {}
```

**parameters** = [ parameter, { ",", parameter } ];

**parameter** = [“&”], type, identifier, [ "=", expression ];

**statement_block** = "{", {statement}, "}";

**statement** = assign_or_call | if_statement | for_statement | switch_statement | declaration, ";" | return_statement | break_statement;

**assign_or_call** = identifier, ("=", expression | "(", arguments, ")"), ";";

```
x = 5;
my_fun(5, 2);
```

**declaration** = type, identifier, [ "=", expression ];

```
bool is_valid = true;
```

**if_statement** = "if", "(", expression, ")", statement_block, [ "else", statement_block ];

```
if (x == 5) {} else {}
```

**for_statement** = "for", "(", [ declaration ], “;”, expression, “;”, [ identifier, "=", expression ], ")", statement_block;

```
for (i64 x = a - 1; i < a + 1; i = i + 1) {}
```

**break_statement** = "break", ";";

```
break;
```

**return_statement** = "return", [ expression ], ";";

```
return a + 2 * b;
```

**argument** = [“&”], expression;

**arguments** = [ argument, {",", argument} ];

```
a + 2, &b, c
```

**expression** = concatenation_term { “||”, concatenation_term };

```
a == b && b || c
```

**concatenation_term** = relation_term, { “&&”, relation_term };

```
a == b && b
```

**relation_term** = additive_term, [ relation_operands, additive_term ];

```
x == y
```

**additive_term** = multiplicative_term , { ("+" | "-"), multiplicative_term };

```
1 + (1 + 2) / (2 + 3)
```

**multiplicative_term** = casted_term, { ("\*" | "/"), casted_term };

```
(1 + 2) / (2 + 3)
```

**casted_term** = unary_term, [ “as”, type ];

```
(x + add(2, 2)) as f64
2 as i64                # 2
2 as f64                # 2.0
2 as str                # “2”
-2 as str               # "-2"
2 as bool               # true
0 as bool               # false
“123” as i64            # 123
“fdsfs” as i64          # błąd
“” as bool              # false
“a” as bool             # true
```

**unary_term** = [ ("-", "!") ], factor;

```
-2
-(x + 5)
!true
```

**factor** = literal | ( "(", expression, ")" ) | identifier_or_call;

```
5
(2.2 + 3 as f64)
x
fun(5)
```

**identifier_or_call** = identifier, [ "(", arguments, ")" ];

```
x
fun(5)
```

**literal** = integer_literal | float_literal | boolean_literal | string_literal;

**identifier** = letter, {character};

```
super_variable_123
```

**switch_statement** = "switch", "(", switch_expressions, ")", "{", {switch_case}, "}";

**switch_expression** = expression, [ ":", identifier ];

**switch_expressions** = switch_expression, { “,”, switch_expression };

**switch_case** = "(", expression, ")", "->", statement_block;

```
switch (x: temp1, y: temp2) {
    (x < 5 && temp2 < 5) -> {
      print("Less than 5.");
    }
    (temp1 < 10 && y < 10) -> {
      print("Less than 10.");
      break;
    }
}
```

### Część leksykalna

**letter** = "a" - "z" | "A" - "Z";

**type** = “i64“| “f64” | “bool” | “str”;

**relation_operands** = "==" | "<" | "<=" | ">" | ">=" | "!=";

**digit** = "0" - “9”;

**non_zero_digit** = "1" - "9";

**integer_literal** = ( non_zero_digit, {digit} ) | “0”;

```
1, 12, 10, 0
```

**float_literal** = integer_literal, ".", {digit}

```
1.0, 1.2, 10.0, 0.0, 0.00001;
```

**string_literal** = “\””, {unicode_character - “\””}, “\””;

**boolean_literal** = “true” | “false”;

**character** = "a" - "z" | "A" - "Z" | "0" - "9" | "\_";

**unicode_character** = (wszystkie znaki unicode)

## Priorytety operatorów

<table>
  <tr>
   <td>operator
   </td>
   <td>priorytet
   </td>
  </tr>
  <tr>
   <td>- (negacja liczby)
   </td>
   <td>7
   </td>
  </tr>
  <tr>
   <td>!
   </td>
   <td>7
   </td>
  </tr>
  <tr>
   <td>as
   </td>
   <td>6
   </td>
  </tr>
  <tr>
   <td>*
   </td>
   <td>5
   </td>
  </tr>
  <tr>
   <td>/
   </td>
   <td>5
   </td>
  </tr>
  <tr>
   <td>+
   </td>
   <td>4
   </td>
  </tr>
  <tr>
   <td>- (odejmowanie)
   </td>
   <td>4
   </td>
  </tr>
  <tr>
   <td>>
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td>>=
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td><
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td><=
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td>==
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td>!=
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td>&&
   </td>
   <td>2
   </td>
  </tr>
  <tr>
   <td>||
   </td>
   <td>1
   </td>
  </tr>
</table>

## Przykłady kodu

**Tworzenie zmiennych**

```
i64 x = 2;
f64 y = 3.0;
bool is_true = false;
str my_string = “hello world”;
str string_with_escapes = "hello \"world"       # hello "world
```

**Wyrażenia**

```
i64 x = 2+2\*2                      # 6
i64 y = (2+2)\*2                    # 8
i64 a = 2 + 2.1 as i64              # 4
f64 b = (2 + 2.1 as i64) as f64     # 4.0
```

**Instrukcje warunkowe**

```
i64 x = 2;
i64 y = 2;
if (x == y) {
    print(“equal”);
} else {
    print(“not equal”);
}
```

**Pętle**

```
for (i64 i = 0; i < 5; i=i+1) {}

i64 j = 0;
for (; j < 5;) {
    j=j+1;
}

for (i64 i = 0; i < 5; i=i+1) {
    if (i == 2) {break;}
}
```

**Funkcje**

```
fn add(i64 x, i64 y): i64 {
    return x + y;
}
add(2, 2);

fn print_int(&i64 x): void {
    print(x as str);
}
print_int(&2);

fn sum_up_to(i64 x): i64 {
    if (x == 0) {return 0;}			# rekurencja
    return x + sum_up_to(x - 1);
}
```

**Pattern matching**

```
switch (x: temp1, y: temp2) {
    (x < 5 && temp2 < 5) -> {
      print("Less than 5.");
    },
    (temp1 < 10 && y < 10) -> {
      print("Less than 10.");
      break;
    }
}
```

## Funkcje wbudowane

1. print(...) - wyświetla argumenty na standardowe wyjście wraz ze znakiem końca linii

## Obsługa błędów

Podział na errory i warningi:

1. errory: zgłaszają błąd komponentowi wyżej, oznaczają brak możliwości kontynuacji działania
2. warningi: komponent wyższego poziomu tworzy funkcję który ma być uruchomiona w razie wystąpienia warninga, działanie nie musi być zatrzymywane

Przykładowe errory:

```
Can’t assign type ‘string’ to type ‘i64’
at line: 5, column: 10
i64 x = “hello world”;
      ^
```

```
Can’t add value of type ‘i64’ to value of type ‘f64’
at line: 10, column: 6
2.1 + 2;
    ^
```

```
Not enough arguments passed - expected 2 given 1
at line: 6, column: 5
add(2);
     ^
```

```
An overflow occurred during integer creation
at line: 10, column: 21
i64 x = 864736473267463264732647326476324;
                                ^
```

Przykładowe warningi:

```
Expected ‘|’
at line: 5, column: 11
```

```
String not closed
at line: 21, column: 37
```

## Sposób uruchomienia

```
cargo run ścieżka_do_pliku
./tkom.exe ścieżka_do_pliku (po zbudowaniu projektu)
```

**Analiza wymagań funkcjonalnych i niefunkcjonalnych**

## Sposób realizacji

Program będzie się składać z analizatora leksykalnego, składniowego i interpretera.

## Opis sposobu testowania

Moduły wymienione w punktach wyżej będą przetestowane testami jednostkowymi, testy integracyjne na całość projektu
