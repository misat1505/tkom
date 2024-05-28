## Funkcjonalność

Język silnie i statycznie typowany.

1. Obsługiwane typy danych:
   - i64 (liczby całkowite)
   - f64 (liczby zmiennoprzecinkowe)
   - str (łańcuchy znaków)
   - bool (prawda / fałsz)
   - void (brak zwracanej wartości z funkcji)
2. Zmienne:
   - Przyjmują jeden z wyżej wymienionych typów oprócz void.
   - Wszystkie są mutowalne.
   - Zmienna widoczna jedynie w bloku instrukcji, w którym została zadeklarowana.
   - Możliwość zadeklarowania zmiennej danego typu bez podania wartości. W takim przypadku zostanie przypisana wartość domyślna dla tego typu.
3. Operacje na Zmiennych:
   - Przypisanie (=)
   - Arytmetyczne (+, -, \*, /)
   - Porównania (==, <, <=, >, >=, !=)
   - Operatory logiczne (||, &&)
4. Konwersja typów:
   - i64 i f64 mogą być castowane na siebie nawzajem, na stringa i na boolean (jeżeli są <= 0 to będzie false inaczej prawda)
   - string może być castowany na i64 i na f64, ze zgłoszeniem błędów, oraz na boolean (pusty string oznacza false inaczej true)
5. Funkcje:
   - Mogą przyjmować parametry przez wartość i referencję
   - Mogą zwracać wartość określonego typu (możliwość funkcji nie zwracającej nic).
   - Funkcje mogą być wywoływane rekurencyjnie
6. Instrukcja if:
   - Opcjonalny else.
7. Pętla for:
   - Iterator nie musi być deklarowany, ani aktualizowany
   - Zadeklarowany iterator nie jest widoczny poza for'em
8. Instrukcja switch (pattern matching):
   - Możliwość zadeklarowania zmiennej widocznej jedynie wewnątrz switch'a
   - Uruchamiany jest każdy blok, dla którego warunek jest spełniony
   - Możliwość przedwczesnego wyjścia przy pomocy break
9. Funkcje wbudowane:
   - print(text): wypisuje string na standardowe wyjście wraz ze znakiem końca linii
   - input(text): wypisuje string na standardowe wyjście i oczekuje na wprowadzenie tekstu od użytkownika, zwraca string
   - mod(a, b): przyjmuje dwie liczby i zwraca wartość a % b

## Przykłady wykorzystania języka

1. Funkcja sprawdzająca czy podana liczba jest liczbą pierwszą. Dodatkowo otrzymuje licznik iteracji przez referencję.

```
fn is_prime(i64 x, &i64 total_iters): bool {
  if (x < 2) {
    return false;
  }

  for (i64 i = 2; i < x; i = i + 1) {
    total_iters = total_iters + 1;
    if (mod(x, i) == 0) {
      return false;
    }
  }

  return true;
}
```

2. Funkcja wyliczająca podany wyraz ciągu Fibonacciego metodą iteracyjną.

```
fn fib_iter(i64 x): i64 {
  i64 prev2 = 1;
  i64 prev1 = 1;
  i64 total;

  if (x == 1 || x == 2) {
    return 1;
  }

  for (i64 i = 2; i < x; i = i + 1) {
    total = prev1 + prev2;
    prev2 = prev1;
    prev1 = total;
  }

  return total;
}
```

3. Funkcja wyliczająca podany wyraz ciągu Fibonacciego metodą rekurencyjną.

```
fn fib_rec(i64 x): i64 {
  if (x == 1 || x == 2) {
    return 1;
  }

  return fib_rec(x - 1) + fib_rec(x - 2);
}
```

4. Program proszący użytkownika o liczbę i wyświetlający informacje o jej znaku.

```
switch (input("Pick a number: ") as i64: x) {
  (x < 0) -> {
    print("Negative");
  }
  (x == 0) -> {
    print("Zero");
  }
  (x > 0) -> {
    print("Positive");
  }
}

```

## Gramatyka

### Część składniowa

**program** = { function_declaration | assign_or_call | if_statement | for_statement | switch_statement | declaration, ";" };

**comment** = "#" , {unicode_character - "\n"}, "\n";

**function_declaration** = “fn”, identifier, "(", parameters, ")", “:”, type | “void”, statement_block;

```
fn is_prime(i64 x, &i64 total_iters): bool {
    return true;
}
```

**parameters** = [ parameter, { ",", parameter } ];

**parameter** = [“&”], type, identifier;

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
for (i64 i = 0; i < 10; i = i + 1) {}
```

```
i64 i = 0
for (; i < 10 ;) {
    i = i + 1;
}
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

## Obsługa błędów

Podział na errory i warningi:

1. errory: zgłaszają błąd komponentowi wyżej, oznaczają brak możliwości kontynuacji działania
2. warningi: komponent wyższego poziomu tworzy funkcję, która ma być uruchomiona w razie wystąpienia warninga, działanie nie jest zatrzymywane

### Błędy lexera

Lexer zgłasza błąd gdy nie może przypisać podanego ciągu znaków do żadnego tokenu. Ponadto, wyłapuje on przepełnienia liczb wpisanych przez uzytkownika oraz za długie komentarze i identyfikatory.

```
Overflow occurred while parsing integer
At line: 13, column: 28

At line:
i64 a = 7647326473264873264873264;
                           ^
```

Lexer zgłasza warningi w przypadku, gdy może się domyśleć o co chodziło użytkownikowi, ale nie zostało przez niego poprawnie napisane.

```
Expected '|'
At line: 18, column: 15
```

```
String not closed
at line: 21, column: 37
```

### Błędy parsera

Parser zgłasza błąd gdy natrafi na token, który nie zgadza się ze specyfikacją gramatyki języka oraz gdy wykryje redeklarację funkcji.

```
Couldn't create statement block while parsing if statement.
At line: 13, column: 12.
```

```
Redeclaration of function 'print'.
At: line: 14, column: 1.
```

### Błędy analizatora semantycznego

Analizator semantyczny zgłasza błąd, gdy znajdzie w drzewie rozbioru wywołanie funkcji nieistniejącej, ze złą liczbą argumentów, albo z argumentami przekazanymi w niewłaściwy sposób.

```
Invalid number of arguments for function 'foo'. Expected 1, given 0.
At line: 18, column: 1.
```

```
Parameter 'x' in function 'foo' passed by Reference - should be passed by Value.
At line: 19, column: 6.
```

### Błędy interpretera

Interpreter zgłasza błąd, gdy natrafi na niedozwoloną operację, tj:

- dodanie wartości różnych typów,
- przypisanie wartości o innym typie niż zmienna,
- przekazanie złego typu jako argumentu funkcji,
- zwrócenie złego typu z funkcji,
- ponowne zadeklarowanie zmiennej,
- gdy warunek w blokach if, switch, for nie są typu bool,
- gdy break jest użyty poza for'em lub switch'em,
- gdy return jest użyty poza funkcją,
- gdy nastąpi przepełnienie stosu wywołań funkcji,
- podczas przepełnienia w operacjach arytmetycznych,
- podczas błędu konwersji typów

```
Cannot perform addition between values of type 'i64' and 'f64'.
At line: 13, column: 13.
```

```
Cannot assign value of type 'str' to variable 's' of type 'i64'.
At line: 18, column: 9.
```

```
Cannot cast String 'abc' to i64.
At: line: 18, column: 9
```

## Sposób uruchomienia

1. Uruchomienie w trybie do debuggowania

```
cargo run ścieżka_do_pliku
```

2. Zbudowanie projektu

```
cargo build --release
.\target\release\tkom.exe ścieżka_do_pliku
```

**Analiza wymagań funkcjonalnych i niefunkcjonalnych**

## Sposób realizacji

Głównymi komponentami programu są analizator leksykalny, składniowy, semantyczny i interpreter.

### Analizator leksykalny

Lexer działa wraz z klasą LazyStreamReader. LazyStreamReader przyjmuje na swoje wejście źródło i oferuje 3 metody:

- current() - zwraca obecny znak
- next() - konsumuje 1 znak i go zwraca
- position() - zwraca obecną pozycję

Lexer pracuje w sposób leniwy - udostępnia metodę 'generate_token'. Odpytuje LazyStreamReader o kolejne znaki i na ich podstawie próbuje stworzyć token. Gdy nie uda mu się stworzyć tokenu z jakielkolwiek kategorii zgłasza błąd.

### Analizator składniowy

Głównym zadaniem parsera jest stworzenie drzewa rozbioru składniowego zgodnego z przyjętą gramatyką. Odpytuje on Lexer o kolejne tokeny poprzez 'generate_token'. Wynikiem jego działania jest drzewo programu podzielone na główne statementy programu, definicję funkcji uzytkownika i funckje wbudowane.

### Analizator semantyczny

Implementuje trait wizytatora, przechodząc wgłąb drzewa szuka wywołań funkcji i sprawdza czy są one poprawne. Jego działanie nie kończy się błędem, ale przechowuje on je w polu wewnątrz siebie.

### Interpreter

Interpreter implementuje trait wizytatora i wykonuje program. W celu komunikacji pomiędzy wizytacjami wprowadzono do interpretera następujące pola:

- 'last_result' - przetrzymuje wyniki pośrednie obliczeń,
- 'last_arguments' - przechowuje argumenty wywołania funkcji,
- 'returned_arguments' - przechowuje wartości argumenty po wykonaniu się funkcji, w celu implementacji referencji.
- flagi 'is_breaking' i 'is_returning', które są zapalane podczas odwiedzin break'a i return'a, a zgaszane zostają przy natrafieniu na konstrukcję umożliwiającą to.

Interpreter współpracuje z klasą Stack, która przechowuje stos wywołań funkcji. Pojedyńczy StackFrame przechowuje instancję klasy ScopeManager'a, która jest również stosem, ale służy ona zarządzania zasięgiem zmiennych. Pojedyńcze pole w stosie ScopeManager'a (Scope) przechowuje HashMap'ę nazwa_zmiennej -> wartość. Wartości reprezentowane są przez enumerację Value, a operacje na nich wykonuje klasa ALU.

## Opis sposobu testowania

1. Testy leksera

   - Sprawdzenie czy poprawnie tworzone są tokeny.
   - Sprawdzenie czy lekser reaguje odpowiednio na niepoprawne wejście.

2. Testy parsera

   - Sprawdzenie poprawności drzewa rozbioru.
   - Sprawdzenie czy parser reaguje na błędy składniowe.

3. Testy interpretera

   - Sprawdzenie czy interpreter poprawnie reaguje na podane drzewa ast.

4. Testy jednostkowe na:

   - LazyStreamReader,
   - ALU,
   - Value,
   - Scope i ScopeManager,
   - Stack

5. Testy integracyjne na całość projektu
