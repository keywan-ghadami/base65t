# Base65t — Spezifikation v0.4

**Status:** aktuell. **Wire-Format: nicht stabil.** v0.4 ersetzt das
Segmentformat der Fassungen v0.1 bis v0.3 durch Blöcke fester Länge, und
nichts verspricht, dass v0.5 die Blöcke behält. Was stabil ist, ist der
Vertrag: Bytes hinein, druckbares ASCII heraus, nie länger als Base64, und
jeder Base64-Strom wird gelesen. Wer heute Ströme speichert, speichert die
Versionsnummer daneben.

Die früheren Fassungen liegen in `docs/history/`, zusammen mit einem
Verzeichnis dessen, was zwischen den Fassungen gestrichen wurde und warum.

**Kurzfassung:** Base64URL, erweitert um ein 65. Zeichen (`~`). Die Eingabe
wird in Blöcke von 48 Bytes geschnitten; ein Block, dessen Bytes das Profil
alle zulässt, steht roh da, jeder andere ist Base64.

> Normative Aussagen sind als solche gekennzeichnet und verwenden MUSS / DARF NICHT /
> SOLLTE nach RFC 2119. Zahlen, die nicht als *exakt* markiert sind, sind Messungen
> auf dem in §16.5 genannten Korpus.

> **Wie die Prozentzahlen zu lesen sind.** Dieses Dokument nennt zwei
> Verhältnisse, und sie zeigen in **entgegengesetzte Richtungen**. Deshalb
> steht bei jeder Zahl, welches gemeint ist, und keine steht ohne diese
> Angabe:
>
> * **Größe** = `len(base65t) / len(base64)`. Weniger ist besser; 100 %
>   heißt gleich groß, und mehr als 100 % ist nach §9.4 unmöglich.
> * **Zeit** = `t(base65t) / t(base64)`. Weniger ist besser; 100 % heißt
>   gleich schnell, mehr als 100 % heißt langsamer.
>
> Wo Zeit gemessen wird, ist die Vergleichsgröße `encode_base64url` bzw.
> `decode` derselben Implementierung auf einem reinen Base64-Strom: dieselbe
> Schleifenform, derselbe Allokator, derselbe Compiler. Ein Vergleich gegen
> eine fremde Base64 würde Handarbeit mitmessen und nicht das Format.

## Änderungen gegenüber der Segmentfassung

Die Abschnittsnummern sind beibehalten, wo der Gegenstand derselbe ist, damit
die Verweise aus `docs/history/` weiter tragen. Wo ein Abschnitt etwas anderes
beschreibt als vorher, steht das am Anfang des Abschnitts.

| § | Änderung |
|---|---|
| 4 | **Blöcke statt Segmente.** Zwei Blockformen fester Länge; keine Längen im Strom |
| 6 | War das Literal-Segment mit Längen-Header; ist jetzt die reservierte Form |
| 9 | Der Encoder ist eine Abbildung je Block, ohne Suche und ohne Zustand. §9.2 (das Programm), §9.2.1 (Fensterung) und §9.2.4 (geschlossene Form) entfallen; §9.6 bleibt, misst aber die eigene Entscheidung statt der Entropie |
| 10 | Der Dekoder kennt vor dem Lesen eines Blocks dessen Länge |
| 10.4 | `E_RESERVED_LEN` und `E_TRUNCATED` entfallen, `E_RESERVED` kommt hinzu |
| 11 | Kanonizität folgt aus der Abbildung; die Ordnung `B < L < S` entfällt |
| 13 | Neu gemessen. Kodieren und Dekodieren liegen in beiden Profilen bei Base64 |
| 14 | Der Dekoder parst keine angreifergewählte Länge mehr |
| 15 | Zwölf Vektoren, neu |

## Was v0.4 zusichert und was nicht

**Größe: zugesichert, je Eingabe, nicht im Mittel.** `len(encode(x)) ≤
ceil(4·len(x)/3)` für jede Eingabe und beide Profile, ohne Ausnahme (§9.4).
Der Beweis ist ein Satz: ein Roh-Block kostet 50 Zeichen, wo Base64 64
kostet, und jeder andere Block *ist* Base64.

**Bytegleichheit: zugesichert.** `encode(x, profil)` ist eine Abbildung, die
je Block aus 48 Bytes eine Ausgabe bestimmt. Es gibt nichts, worüber zwei
Encoder verschiedener Meinung sein könnten (§11).

**Durchsatz: eine Zusicherung der Form, wo nicht kleiner, dann gleich.** Wo
die Ausgabe nicht kürzer ist als Base64, ist sie Base64 — dieselben Bytes und
dieselbe Zeit (§9.6, §13). Wo sie kürzer ist, ist sie meist auch schneller.
Eine Zahl für jeden Fall zusichern kann die Spezifikation nicht; §13 misst.

**Das Wire-Format: nicht zugesichert.** Siehe oben.

## 0. Positionierung (nicht normativ)

### 0.1 Ein Format, ein Encoder

Base65t ist für den Fall gedacht, in dem jemand **unsicher** ist: er hat
etwas, das ohnehin schon Text ist, und muss es durch einen Kanal schicken, der
Bytes akzeptieren muss. Base64 kann er, Base64 versteht jeder, Base64 ist
niemals falsch. Alles, was er über ein anderes Verfahren erst lernen muss,
ist ein Grund, es bleiben zu lassen.

Deshalb gibt es genau eine parameterlose `encode`-Funktion, die Bytes nimmt
und Bytes liefert. Es gibt keine Presets, keine Modi, keine Schwellwerte. Der
Encoder ist in einem Satz erklärt: **48 Bytes Text bleiben Text, alles andere
ist Base64.**

Eine dritte Blockform hat es einen Tag lang gegeben: eine Maske, die je Byte
sagte, welche Bytes eines gemischten Blocks im Klartext stehen. Sie war
gemessen dreimal so teuer wie Base64 auf jedem Block, für den sie galt.
`docs/history/` beschreibt sie; §17 hält die Tür für sie offen.

Zwei Parameter bleiben, und keiner davon ist eine Wahl über die Kodierung:

* **Das Profil** (§7) ist eine Aussage über den *Container*, nicht über den
  Strom, und aus dem Strom nicht ableitbar (§7.2). Der Default ist U.
* **`encode_base64url`** (§9.3) ist kein Modus des Formats, sondern der Ausgang
  aus ihm: für einen Aufrufer, der ein Geheimnis trägt und nichts davon im
  Klartext stehen haben will (§14), und für einen, der nur Base64URL sprechen
  darf.

| Einsatz | Profil | worauf es ankommt |
|---------|--------|-------------------|
| URL-Query | U | URL-Sicherheit ohne Prozent-Encoding |
| Cookie-Wert | U | `cookie-octet`-Konformität (§7.1) |
| HTTP-Header | U oder T | ASCII, keine Trennzeichen |
| Cache-/Dedup-Key | wie der Container | Bytegleichheit (§11) |
| Log-Feld, JSON-String | T | dort bleibt der Text lesbar (§13.4) |
| Token mit Geheimnisanteil | — | `encode_base64url`, keine Klartext-Leaks (§14) |

### 0.2 Was Base65t *ist*, in einem Satz

> Base64URL in Blöcken von 48 Bytes, wobei ein Block seine Bytes roh tragen
> darf und ein 65. Zeichen sagt, welche Blöcke das sind.

### 0.3 Was „ein Dekoder für alles" genau heißt

> Ein konformer `decode()` nimmt einen Octet-Stream und ein Profil entgegen und
> benötigt **keinen weiteren Parameter**. Alphabetvariante (§5.2) und Padding
> (§5.3) werden aus dem Strom selbst bestimmt und im Ergebnis gemeldet.

### 0.4 Warum nicht „Base85N mit URL-Alphabet"

Die *unreserved*-Menge von RFC 3986 umfasst 66 Zeichen. Eine Radix-85-Kodierung ist
darin darstellbar, aber ein Passthrough vom Typ Base85N braucht zusätzlich
Spender-Zeichen — dafür bleibt kein Spielraum, sobald 64 Zeichen für den
Binärkern gebunden sind. Base65t geht den entgegengesetzten Weg: ein Kern, der
**exakt Base64URL ist**, plus ein Diskriminator. Nur daraus folgt die
Superset-Eigenschaft aus §5.2.

### 0.5 Wo Base65t in der Familie steht (nicht normativ)

Base65t ist der **Wegbereiter**, nicht der Höhepunkt. Base85N führt denselben
Durchreich-Gedanken weiter und ist dichter; Base91z komprimiert. Beide
verlangen vom Aufrufer, ein Verfahren zu lernen, das er noch nicht kennt.
Base65t verlangt das nicht: der Kern *ist* Base64URL, die Ausgabe ist niemals
größer und niemals nennenswert langsamer, und die einzige Frage, die bleibt,
ist das Profil.

Der Mehrwert ist entsprechend klein — auf kurzen Werten 21 % — und die Kosten
sind es auch. Das ist die Absicht: wer den ersten Schritt macht, soll ihn
ohne Abwägung machen können. Wer danach gemischten Text lesbar haben will
oder Dichte braucht, geht eine Tür weiter.

## 1. Zielsetzung

1. **Nie schlechter als Base64** (§9.4).
2. Text durchreichen, wo er in Stücken von 48 Bytes zusammenhängt (§13.4).
3. Lesbar bleiben.
4. **Kein Escaping** — auch nicht für `~`.
5. **Abwärtskompatibel lesen** — jeder kanonische Base64- oder Base64URL-Strom,
   gepaddet oder nicht, dekodiert zu denselben Bytes (§5.2, §5.3). Normativ.
6. **Selbstbestimmend im Strom** — Alphabet und Padding werden erkannt,
   nicht konfiguriert (§0.3).
7. Bytegleich reproduzierbar (§11).
8. **Zustandslos.** Kein Block hängt von einem anderen ab (§4).
9. **Nicht nennenswert langsamer als Base64**, in beide Richtungen (§13).

### 1.1 Die Kompatibilität ist asymmetrisch

| Richtung | Gilt? |
|----------|-------|
| Base65t-Dekoder liest Base64URL, ungepaddet | **ja**, normativ |
| Base65t-Dekoder liest Base64URL, gepaddet | **ja**, normativ, §5.3 |
| Base65t-Dekoder liest klassisches Base64 (`+`/`/`), gepaddet oder nicht | **ja**, normativ, §5.2/§5.3 |
| **Base64-Dekoder liest Base65t** | **nein** — `~` ist nicht im Alphabet |
| **Base65t v0.4 liest v0.1 bis v0.3** | **nein** — anderes Wire-Format |

Base65t ist ein *Superset auf der Leseseite* gegenüber Base64. Migrationspfad:
erst Dekoder ausrollen, später Encoder umstellen.

**Kanonizität der Eingabe.** Die Aussage gilt für *kanonische* Ströme. Ein
Base64-Strom mit gesetzten Restbits (`YWxpY2V`) wird mit `E_NONZERO_TAIL`
abgewiesen — auch dann, wenn eine permissive Base64-Bibliothek ihn akzeptiert
hätte. Das ist Absicht und gehört in den Differential-Fuzzing-Korpus (§16.2)
als *erwartete Abweichung*.

## 2. Nicht-Ziele

* **Kein Kompressionsformat.** Ab ca. 1 KB Text schlägt `gzip` + Base64 deutlich.
* **Kein Dichte-Rekord.** Z85, basE91, Base85N sind binär dichter.
* **Kein Durchsatz-Rekord.** Base64 ist der Maßstab, und Dichte wird nie gegen ihn
  eingetauscht.
* **Kein Sicherheitsmechanismus.**

## 3. Notation

* `byte` = Byte der Nutzdaten. `char` = Zeichen des Ausgabestroms.
* **Base65t erzeugt einen Octet-Stream.** In beiden Profilen ist jedes Oktett
  druckbares ASCII.
* Kernalphabet nach Base64URL (RFC 4648 §5): 0–25 `A`–`Z`, 26–51 `a`–`z`,
  52–61 `0`–`9`, 62 `-`, 63 `_`.
* Das 65. Zeichen ist `~` (U+007E), nicht Teil des Alphabets, ohne Wert.
* Bitreihenfolge MSB-first. MUSS / DARF NICHT / SOLLTE nach RFC 2119.

**Alphabetzeichen.** Ein Oktett heißt *Alphabetzeichen*, wenn der Dekoder es als
Wert 0–63 interpretiert: die Zeichen von Base64-Läufen — **nicht** die rohen
Bytes eines Blocks. Tragend für §5.4.

## 4. Streamstruktur

```
Stream      := Block*
Block       := Base64Block | RawBlock
Base64Block := <64 Alphabetzeichen>                        # 48 Bytes
RawBlock    := "~~" <48 rohe Bytes>
```

**Die Eingabe wird an absoluten Offsets `k · 48` geschnitten.** Jeder Block
außer dem letzten deckt genau 48 Eingabebytes ab; der letzte deckt die
restlichen `m ≤ 48` und ist entsprechend kürzer: ein Base64-Tail hat
`ceil(4m/3)` Zeichen, ein Raw-Tail `2 + m` und läuft bis zum Stromende.

**Blöcke sind unabhängig (normativ).** Die Kodierung eines Blocks hängt nur
von seinen eigenen Bytes ab. Daraus folgt alles, was dieses Format von seinen
Vorgängern unterscheidet: der Encoder hat keinen Zustand, der Dekoder kennt
die Länge eines Blocks, bevor er ihn liest, und ein Strom lässt sich an jeder
Blockgrenze teilen und wieder zusammensetzen.

**Base64-Blöcke kacheln.** 48 Bytes sind 16 Quanten, also endet ein
Base64-Block auf einer Quantengrenze, und zwei aufeinanderfolgende
Base64-Blöcke sind ein Base64-Lauf. Deshalb ist ein reiner Base64-Strom ein
gültiger Strom, und deshalb DARF ein Dekoder aufeinanderfolgende
Base64-Blöcke als einen Lauf lesen (§10.1).

**`~` gefolgt von einem Alphabetzeichen ist reserviert (normativ).** Kein
Encoder schreibt es, und ein Dekoder MUSS es mit `E_RESERVED` abweisen. Damit
kann eine spätere Fassung eine dritte Blockform einführen, ohne dass ein
Dekoder dieser Fassung sie stillschweigend falsch liest (§17).

**Warum 48.** Drei Bedingungen: durch 3 teilbar, damit Base64-Blöcke kacheln;
durch 6 teilbar, was die reservierte Form aus §17 brauchen würde; und groß
genug, dass die zwei Marker-Zeichen eines Raw-Blocks vier Prozent davon sind
und nicht ein Drittel — bei sechs Bytes je Block spart die rohe Form genau
nichts (§9.1). Größere Blöcke sparen wenig zusätzlich und kippen häufiger
ganz nach Base64, weil ein einziges profilwidriges Byte genügt.

Ein leerer Strom ist gültig und dekodiert zu null Bytes.

## 5. Base64-Läufe

Base64URL. Sei `n` die Zeichenzahl ohne Padding:

| `n mod 4` | Bytes im letzten Quantum | Gültig |
|-----------|--------------------------|--------|
| 0 | 3 | ja |
| 2 | 1 | ja |
| 3 | 2 | ja |
| 1 | — | **nein** (`E_ALIGN`) |

**Kanonizität:** ungenutzte Bits im letzten Zeichen MÜSSEN 0 sein
(`E_NONZERO_TAIL`).

### 5.1 Encoder-Alphabet

Ein Encoder MUSS genau ein Alphabet je Aufruf verwenden und DARF innerhalb eines
Stroms nicht wechseln.

| Alphabet | 62 / 63 | Zweck |
|----------|---------|-------|
| **URL** (Default) | `-` / `_` | alles Neue |
| **Classic** (Opt-in) | `+` / `/` | nur Interop |

Classic ist nicht URL-sicher und DARF NICHT als Default angeboten werden. Der
Encoder erzeugt **niemals** Padding (§5.3).

### 5.2 Permissive Dekodierung (normativ)

Jeder konforme Dekoder MUSS beide Alphabete akzeptieren: `-`/`+` → 62, `_`/`/` → 63.

### 5.3 Padding (normativ)

> **Regel P.** Ein Base64-Lauf, der **am Stromende** endet, DARF mit 1 oder 2 `=`
> abgeschlossen sein. Der Dekoder MUSS diese akzeptieren. „Strom" heißt der
> ganze Oktett-Strom.

| `k` (`=`-Anzahl) | erforderlich |
|------------------|--------------|
| 0 | `n mod 4 ∈ {0, 2, 3}` |
| 1 | `n mod 4 == 3` |
| 2 | `n mod 4 == 2` |

Jede andere Kombination → `E_PADDING`. `=` an jeder anderen Position →
`E_CHARSET`.

**Implementierungsfalle.** Padding DARF NICHT vorab vom Stromende gestrippt
werden. In Profil T ist `=` legales rohes Byte, und ein Raw-Tail läuft bis
zum Stromende: `~~a=b=` sind vier Bytes Nutzlast (TV10).

### 5.4 Alphabet-Konsistenz (Regel A, normativ)

> **Regel A.** Ein Strom DARF NICHT beide Alphabetvarianten mischen. Enthält die
> Menge der **Alphabetzeichen** sowohl ein Zeichen aus {`+`,`/`} als auch eines aus
> {`-`,`_`} → `E_MIXED_ALPHABET`. Die Regel gilt über den ganzen Strom, also
> auch über die Raw-Blöcke hinweg, die zwischen zwei Base64-Blöcken stehen.

Regel A betrifft ausschließlich Alphabetzeichen. Rohe Bytes zählen nicht mit
— in Profil U enthält fast jeder Raw-Block `-` oder `_`. Wer den Gesamtstrom
scannt, weist gültige Ströme ab (TV7).

### 5.5 Meldung und strikte Variante (normativ)

Ein `decode()`-Ergebnis MUSS enthalten:

```
alphabet_seen : { none, url, classic }
padding_seen  : bool
```

Zusätzlich MUSS `decode_url_strict` angeboten werden (weist `classic` mit
`E_NON_URL_ALPHABET` ab).

## 6. Die reservierte Form

```
"~" <Alphabetzeichen> ...          # reserviert, E_RESERVED
```

Kein Encoder dieser Fassung schreibt `~` gefolgt von einem Alphabetzeichen,
und ein Dekoder MUSS es abweisen. Die zwei Zeichen kosten heute nichts und
halten die Tür offen: v0.4 hatte für einen Tag eine dritte Blockform, die
hier ansetzte — `~`, acht Maskenzeichen mit einem Bit je Byte, die zulässigen
Bytes im Klartext, dann Base64 des Rests. Sie hat einen gemischten Block zu
zwei Dritteln lesbar gemacht und dafür das Dreifache der Base64-Zeit gekostet
(`docs/history/`).

Der Grund, sie zu streichen, ist §0.1: das Format lebt davon, dass die
Entscheidung dafür nichts kostet. „Dreimal langsamer auf meinen JSON-Blobs"
ist ein Satz, der genau diese Entscheidung kippt, und lesbarer gemischter
Text ist nicht das, wofür das Format wirbt.

`~` gefolgt von etwas, das weder `~` noch ein Alphabetzeichen ist, ist kein
reservierter, sondern ein kaputter Strom: `E_CHARSET`.

## 7. Profile

| Profil | Erlaubte rohe Bytes | URL-Query direkt? |
|--------|---------------------|-------------------|
| **U** (Default) | RFC-3986-*unreserved* (66 Zeichen) | **ja** |
| **T** | ASCII 0x20–0x7E ohne `"` und `\` (93 Zeichen) | nein |

Ein profilwidriges Byte kostet seinen ganzen Block: der Block wird Base64.
Das ist die Grobheit, die dieses Format gegen die Geschwindigkeit
eingetauscht hat, und §13.4 beziffert sie.

**Profil T** ist JSON-String-sicher, **nicht** CSV-struktursicher und **nicht**
URL-sicher. **Und es enthält das Leerzeichen** (0x20): eine
whitespace-getrennte Logzeile muss einen T-Wert quoten, ein
`key=value`-Format nicht. Gefunden vom Container-Test aus §16.6.

### 7.1 Cookie-Konformität von Profil U (bewiesen, nicht gemessen)

RFC 6265 §4.1.1 definiert:

```
cookie-octet = %x21 / %x23-2B / %x2D-3A / %x3C-5B / %x5D-7E
```

Das Alphabet von Profil U — 62 Alphanumerische plus `-` (0x2D), `.` (0x2E),
`_` (0x5F), `~` (0x7E) — liegt vollständig in diesen Bereichen. Alle 66
Zeichen geprüft, keine Ausnahme. Die Aussage folgt aus der ABNF und ist damit
**beweisbar, nicht empirisch**. Ob reale Cookie-Parser sich an die ABNF
halten, ist die schwächere, empirische Frage; Pythons `http.cookies` tut es
(§16.6).

### 7.2 Warum das Profil Parameter bleibt

Das Profil ist aus dem Strom nicht ableitbar: ein Strom, dessen rohe Bytes
zufällig nur *unreserved* sind, ist unter U und T identisch gültig. Es
beschreibt die Erwartung des **Containers**, nicht eine Eigenschaft des
Stroms.

## 8. Framed Mode — **zurückgezogen**

Siehe `docs/history/`. Ein Blockformat mit festen Blockgrenzen braucht keinen
zweiten Modus für wahlfreien Zugriff; wer ihn will, indiziert Blockanfänge,
und das ist ein Erweiterungskandidat (§17), keine Formatfrage.

## 9. Encoder

### 9.0 Grundprinzip (normativ)

> Für jeden Block prüft der Encoder, ob das Profil **jedes** seiner Bytes
> zulässt. Wenn ja und der Block mindestens vier Bytes hat, schreibt er `~~`
> und die Bytes; sonst schreibt er den Block als Base64.

Das ist die ganze Regel. Sie ist eine Abbildung von 48 Bytes und einem Profil
auf eine Ausgabe, ohne Suche, ohne Zustand und ohne Gleichstand, den eine
Ordnung auflösen müsste. Damit prüft ein Testvektor Bytes statt Längen, und
`docs/vectors.json` tut das über 173 Vektoren.

Die vier Bytes sind kein Schwellwert, sondern die Stelle, an der die rohe Form
aufhört, teurer zu sein: siehe §9.1.

### 9.1 Was die Formen kosten

Für einen Block von `m` Bytes:

```
Base64:  ceil(4m/3)
Raw:     m + 2                          nur wenn jedes Byte zulässig ist
```

| `m` | Base64 | Raw | |
|--:|--:|--:|---|
| 1 | 2 | 3 | Base64 |
| 3 | 4 | 5 | Base64 |
| 4 | 6 | 6 | Gleichstand → Raw |
| 6 | 8 | 8 | Gleichstand → Raw |
| 7 | 10 | 9 | Raw |
| 48 | 64 | 50 | Raw, 78 % |

Ab vier Bytes ist die rohe Form nie länger, und bei vier, fünf und sechs
genau gleich lang; dort nimmt der Encoder sie trotzdem, weil ein Gleichstand
nichts kostet und Text im Klartext ist, wofür das Format da ist. Der Gewinn
wächst mit der Blockgröße und läuft gegen `(m+2)/(4m/3)`, also gegen 75 %; bei
48 Bytes sind 78 % erreicht.

**Alles oder nichts.** Ein einziges profilwidriges Byte kostet seinen ganzen
Block. Das ist grob, und es ist der Tausch, den diese Fassung macht: eine
feinere Kodierung — eine Maske je Byte — gab es einen Tag lang und kostete
das Dreifache der Base64-Zeit (§6, `docs/history/`). Was die Grobheit auf
echten Daten bedeutet, steht in §13.4: kurze Werte, die ganz aus Text
bestehen, holen 78 %; große Dokumente holen in Profil U **nichts** und in
Profil T fünf bis zehn Prozent.

### 9.2 Optimale Segmentierung — **entfällt**

Es gibt nichts zu segmentieren.

### 9.3 Einstiegspunkte

| Funktion | Was sie tut | Profil |
|---|---|---|
| `encode(x)` | die Kodierung | U |
| `encode(x, profil)` | dieselbe, im Profil, das der Container verlangt | U oder T |
| `encode_base64url(x)` | Base64URL und sonst nichts (§14) | — |

Ein Aufruf ohne Parameter MUSS `encode` + Profil U liefern. Bibliotheken
SOLLTEN genau eine parameterlose `encode`-Funktion exportieren.

### 9.4 Nie-schlechter-Garantie (normativ)

```
len(encode(x, profil)) <= ceil(4 * len(x) / 3)
```

**Je Eingabe, nicht im Mittel, ohne Ausnahme.** Beweis: ein Roh-Block kostet
`m + 2 ≤ ceil(4m/3)` (§9.1), jeder andere Block *ist* Base64, und
Base64-Blöcke kacheln, also ist die Summe der Base64-Formen genau
`ceil(4n/3)`. ∎

**Schärfer:** wo kein Block roh wird, schreibt der Encoder nicht nur gleich
viele Zeichen wie Base64URL, sondern **dieselben Bytes**.

**Geltungsbereich.** Die Länge des kodierten Stroms in Oktetten, nicht
Transport- oder Container-Overhead.

### 9.5 Segmentwechselrate — **entfällt**

Es gibt keine Segmentwechsel.

### 9.6 Die Stichprobe (normativ)

Die Frage aus §9.0 je Block zu stellen kostet Zeit, und auf einer Eingabe, in
der die Antwort immer „nein" lautet, ist es die einzige Zeit, die dieses
Format über Base64 hinaus verbraucht. Solche Eingaben sind nicht nur
Binärdaten: englische Prosa in Profil U hat in jedem Block ein Leerzeichen,
also wird kein Block roh, und sie ist für dieses Format so binär wie ein JPEG.

> **Regel.** Vor dem Kodieren wendet der Encoder §9.0 auf die ersten **64
> Blöcke** an. Ergibt keiner davon die rohe Form, MUSS der ganze Strom als
> Base64URL geschrieben werden. Andernfalls gilt §9.0 für jeden Block.

**Es ist dieselbe Prüfung, einmal vorab.** Keine Magic Numbers, keine
Entropie, kein Logarithmus, über den zwei Implementierungen sich einig werden
müssten — die Stichprobe misst die Entscheidung selbst und nicht etwas, das
mit ihr korreliert. Frühere Fassungen taten das andere; `docs/history/`
beschreibt sie.

**Die Ausgabe bleibt eine Funktion der Eingabe.** Die Stichprobe ist ein
fester Präfix, die Zahl der Blöcke ist eine Konstante, und die Prüfung ist
die aus §9.0. §9.0 gilt unverändert.

**Eine falsche Entscheidung kostet Größe, nie Korrektheit.** Ein übersprungener
Strom ist exakt Base64URL, also greift §9.4 in jedem Fall.

**Warum 64 Blöcke.** Zwei Gründe, die beide zählen. Gemessen ist es das Knie:
bei 32 Blöcken wird `xml` unter Profil T falsch eingeschätzt und gibt 9,8
Punkte auf fünf Megabyte auf, bei 64 nicht, und darüber bewegt sich fast
nichts mehr, während weniger Ströme den billigen Weg nehmen. Und 64 Blöcke
sind **3072 Bytes** — länger als jeder Wert, den §0.1 nennt. Für eine
URL-Query, einen Cookie-Wert, einen Header oder einen Cache-Key ist die
Stichprobe deshalb keine Stichprobe, sondern die ganze Eingabe, und sie kann
dort nichts aufgeben.

**Was sie über den Korpus kostet**, gegen „immer prüfen"
(`binary2textbench`, `--example sample`, 101 Proben):

| | Profil U | Profil T | als reines Base64 geschrieben |
|---|--:|--:|--:|
| immer prüfen | 99,95 % | 97,40 % | — |
| Stichprobe, 64 Blöcke | 99,99 % | 97,50 % | 67 bzw. 37 von 101 |

Vier Hundertstel Prozentpunkt in U und ein Zehntel in T. Dafür werden in
Profil U zwei Drittel aller Dateien byteweise als Base64 geschrieben, in
Base64s Zeit.

## 10. Dekoder

### 10.1 Ablauf

```
pos := 0 ; alphabet_seen := none ; padding_seen := false
while pos < len:
    if stream[pos] != '~':
        # Base64-Lauf: jeder Block, der mit einem Alphabetzeichen beginnt,
        # ist 64 Zeichen lang; der letzte, was übrig ist. Blöcke kacheln,
        # also DARF der Lauf als Ganzes dekodiert werden.
        end := pos
        while end < len and stream[end] != '~': end := min(end + 64, len)
        emit base64_decode(stream[pos..end], padding_erlaubt = (end == len))
        pos := end
    elif pos + 1 == len:                                  -> E_TRAILING_TILDE
    elif stream[pos+1] == '~':
        # Raw-Block: 48 Bytes, oder was übrig ist.
        end := min(pos + 2 + 48, len)
        prüfe: alle Bytes stream[pos+2..end] profil-legal    sonst E_PROFILE
        emit stream[pos+2..end] ; pos := end
    elif stream[pos+1] ist Alphabetzeichen:               -> E_RESERVED       # §6
    else:                                                 -> E_CHARSET

base64_decode(seg, padding_erlaubt):                       # §5, §5.3
    k := padding_erlaubt ? Anzahl '=' am Ende (max 2) : 0
    n := len(seg) − k
    prüfe: k == 0 ∨ (k == 1 ∧ n mod 4 == 3) ∨ (k == 2 ∧ n mod 4 == 2)
                                                             sonst E_PADDING
    if k > 0: padding_seen := true
    prüfe: n mod 4 != 1                                      sonst E_ALIGN
    prüfe: alle n Zeichen Alphabetzeichen                    sonst E_CHARSET
    note_alphabet für jedes Zeichen mit Wert 62/63
    prüfe: Restbits des letzten Quantums == 0                sonst E_NONZERO_TAIL
    return Bytes

note_alphabet(c):
    if c in {'+','/'}:  if alphabet_seen == url     -> E_MIXED_ALPHABET
                        else alphabet_seen := classic
    if c in {'-','_'}:  if alphabet_seen == classic -> E_MIXED_ALPHABET
                        else alphabet_seen := url
```

**Es gibt keine Suche und keine Länge.** Der Dekoder liest nie „bis zum
nächsten `~`". Jede Blocklänge steht fest, bevor er ein Nutzbyte anfasst, und
keine davon steht im Strom. Das ist mehr als eine Bequemlichkeit: ein Byte
`~` in einem Raw-Block ist Nutzlast, und ein Dekoder, der danach sucht, liest
ihn falsch (TV3).

**Warum der Tail eindeutig ist.** Ein Raw-Tail läuft bis zum Stromende, ein
Base64-Tail ebenso. „Es bleiben weniger Zeichen als ein voller Block braucht"
ist die ganze Tail-Erkennung, und weil kein Block eine Länge ankündigt, kann
auch nichts abgeschnitten sein: ein gekürzter Strom dekodiert zu einem Präfix
der Eingabe oder scheitert an Regel P, nicht an einer Längenangabe.

### 10.2 Einstiegspunkt

```
decode(stream, profile)
decode_url_strict(stream, profile)      # weist '+' und '/' mit E_NON_URL_ALPHABET ab
```

### 10.3 Framed Mode — **zurückgezogen**

Siehe §8.

### 10.4 Fehlerfälle

| Code | Bedingung |
|------|-----------|
| `E_TRAILING_TILDE` | Strom endet mit einem einzelnen `~` |
| `E_RESERVED` | `~` gefolgt von einem Alphabetzeichen (§6) |
| `E_PROFILE` | rohes Byte außerhalb des Profil-Alphabets |
| `E_ALIGN` | Base64-Lauflänge `mod 4 == 1` |
| `E_NONZERO_TAIL` | Restbits im letzten Quantum ≠ 0 |
| `E_CHARSET` | kein Alphabetzeichen an Alphabetposition (inkl. `~` in einem Base64-Lauf, `=` außerhalb des Stromendes, und `~` gefolgt von einem wertlosen Zeichen) |
| `E_PADDING` | Regel P verletzt |
| `E_MIXED_ALPHABET` | Regel A verletzt |
| `E_NON_URL_ALPHABET` | nur `decode_url_strict` |

Neun Codes. `E_TRUNCATED` gibt es nicht mehr, weil es nichts gibt, was
abgeschnitten sein könnte.

**Allokationsgrenzen.** Es gibt im Strom keine Länge, die ein Sender wählt.
Ein Raw-Block hat höchstens 48 Bytes, ein Base64-Lauf ergibt drei Bytes je
vier Zeichen. Daraus folgt: die Spezifikation braucht **kein
protokollseitiges Limit für einzelne Blöcke**, und die Klasse der
Einzelallokations-Bugs, die §14 der Segmentfassung als ihre eine Schwäche
gegenüber Base64 nannte, existiert nicht. Die Zahl der Blöcke ist unbegrenzt;
Implementierungen SOLLTEN Gesamtgrößen- und Laufzeitlimits anbieten.

## 11. Kanonizität und Signaturen

**Der Encoder ist eine Abbildung** (§9.0): je Block bestimmen 48 Bytes und
das Profil die Ausgabe, und die Blöcke sind unabhängig. Zwei konforme Encoder
schreiben für dieselbe Eingabe und dasselbe Profil dieselben Bytes. Das reicht
für Cache-Keys, Dedup-Keys und Content-Adressen, wo dieselbe Seite erzeugt
und vergleicht.

Kanonisch ist das *Format* trotzdem nicht, aus zwei Gründen. Erstens ist das
**Profil eine Wahl**: derselbe Input ergibt unter U und T verschiedene Ströme.
Zweitens akzeptiert der **Dekoder Formen, die kein Encoder schreibt**: das
Classic-Alphabet (§5.2), Padding (§5.3), und einen Base64-Block, wo ein
Raw-Block kürzer wäre. Ein Dritter kann denselben Strom umschreiben, ohne die
dekodierten Bytes zu ändern.

> **Regel:** Signiere, hashe und vergleiche niemals die Ausgabe von `encode`.
> Signiere die **dekodierten Bytes**. `decode(encode(x)) == x` gilt immer.

**Die Ordnung `B < L < S`** der Segmentfassung gibt es nicht mehr. Sie war
nötig, weil dort mehrere Segmentierungen gleich lang sein konnten und eine
davon gewählt werden musste. Hier gibt es je Block zwei Formen und eine
Bedingung, die entscheidet.

## 12. Dichte

**Exakt**, aus §9.1:

Zeichen je Eingabebyte; weniger ist besser.

| Eingabe | Base64 | **Base65t** |
|---------|--------|-------------|
| Ein Block mit einem profilwidrigen Byte | 1,333 | **1,333** — dieselben Bytes |
| Rein profil-legaler Text | 1,333 | **1,0417** — `50/48`, jeder Block roh |

Dazwischen gibt es nichts: ein Block ist das eine oder das andere. Was eine
Datei erreicht, hängt daher nur daran, wie viele ihrer 48-Byte-Blöcke ganz
aus zulässigen Bytes bestehen.

**Gemessen** über den Korpus von binary2textbench (69 Proben, `--example
gain`), Größe gegen ungepaddetes Base64:

| | Profil U | Profil T |
|---|--:|--:|
| Summe über alle Proben | 99,99 % | 99,51 % |
| Proben besser als 95 % | 43 % | |
| von Base64 nicht zu unterscheiden (≥ 99,9 %) | 55 % | |

**Die Summenzeile ist ehrlich und irreführend zugleich**, weil sie nach Bytes
gewichtet ist und der Korpus von Megabyte-Dateien bestimmt wird. Auf denen
holt diese Fassung fast nichts: ein Dokument mit Leerzeichen alle fünf
Zeichen hat in Profil U keinen einzigen ganz zulässigen Block. Die Verteilung
ist zweigeteilt, und die interessante Hälfte ist die kleine:

| Probe | Bytes | Profil U |
|---|--:|--:|
| Git-Commit-ID | 40 | **77,8 %** |
| Session-ID, 40 alnum | 40 | **77,8 %** |
| SHA-512-Digest, hex | 128 | **78,4 %** |
| zwei UUIDs | 73 | **78,6 %** |
| JWT, drei Segmente | 155 | **78,7 %** |
| Prosa, XML, JSON, jede Megabyte-Datei | | 100,0 % |

**Gegenüber den früheren Fassungen**, dieselben Proben: die Segmentfassung
kam auf 98,57 % in U, die Maskenfassung auf 98,65 %. Diese Fassung ist auf
großen Dateien schlechter und auf kurzen Werten gleich gut — und §13 sagt,
was sie dafür bekommt. Von der Differenz gehen 0,01 Punkte in U und 0,24 in T
auf die Stichprobe aus §9.6; der Rest ist die Grobheit des Blocks selbst.

## 13. Performance

Gemessen gegen die Base64-Implementierung des Benches, die im selben Prozess
lebt und vom selben Compiler gebaut wurde. Alles einthreadig, bestes von fünf
Läufen, Base64 = 100 %.

### 13.1 Was die Prüfung kostet, und wo sie nicht anfällt

Ein Raw-Block ist ein `memcpy` in beide Richtungen, ein Base64-Block ist
Base64. Der einzige Aufwand, den dieses Format über Base64 hinaus hat, ist die
Frage je Block: lässt das Profil **jedes** Byte zu?

Sie ist so billig gebaut, wie es ohne Vektorbefehle geht — sie bricht beim
ersten Byte ab, das sie entscheidet, prüft eine notwendige Bedingung vorweg
mit einer einzigen Operation je 32 Bytes, und ihr Test je Byte ist Arithmetik
statt Tabellenzugriff, weil ein Gather nicht vektorisiert und sechs Vergleiche
schon. Gemessen gegen `encode_base64url` derselben Implementierung, Median
gepaarter Quotienten:

| Eingabe je 48-Byte-Block | nur die Prüfung | Kodieren gesamt |
|---|--:|--:|
| ganz zulässig (Raw) | 33 % | **48 %** |
| Binärdaten | 6 % | 109 % |
| Text, abweisendes Byte am Blockende | 34 % | 145 % |

**Und dann fällt sie meistens gar nicht an.** Die zweite und die dritte Zeile
sind genau die Fälle, in denen die Stichprobe aus §9.6 „nein" sagt: der Strom
wird als Base64URL geschrieben, kein Block wird geprüft, und die Zeile
verschwindet aus der Rechnung. Übrig bleibt die erste — die, in der das Format
etwas holt und schneller ist als Base64.

Beim **Dekodieren** gibt es diesen Aufwand nie: die Form steht im ersten
Zeichen.

### 13.2 Das Durchsatz-Kriterium

> **Durchsatz ist ein Ziel, Größe ist eine Zusicherung.** Eine Änderung DARF
> die Zusicherung aus §9.4 und die Bytegleichheit aus §11 nicht antasten.
> Innerhalb dieser Schranke SOLL sie den Durchsatz verbessern.

Ein Encoder oder Dekoder ist nicht deshalb unkonform, weil er langsamer ist als
Base64. Er ist es, wenn er die falschen Bytes schreibt.

### 13.3 Große Dateien

Gegen `encode_base64url` derselben Implementierung — dieselbe Schleifenform,
derselbe Allokator —, Median gepaarter Quotienten über 21 Runden:

| Datei | Profil | Größe | Kodieren | Dekodieren |
|---|---|--:|--:|--:|
| erzeugt, ganz zulässig | U | 78,1 % | **48 %** | **40 %** |
| Prosa, Leerzeichen alle 6 Bytes | T | 78,1 % | **40 %** | **35 %** |
| `xml` | T | 88,4 % | **91 %** | **68 %** |
| `dickens` | T | 95,1 % | 118 % | **90 %** |
| `dickens` | U | 100,0 % | **100 %** | **100 %** |
| `xml` | U | 100,0 % | **99 %** | **100 %** |
| `countries.json` | U | 100,0 % | **100 %** | 101 % |
| `x-ray` (binär) | U | 100,0 % | 101 % | 101 % |
| Zufallsbytes | U | 100,0 % | **100 %** | 101 % |

**Die Tabelle ist nach Größe sortiert, und das ist die ganze Aussage.** Wo die
Ausgabe nicht kürzer ist als Base64, ist sie Base64 und kostet dessen Zeit:
99 bis 101 %, in beide Richtungen. Wo sie kürzer ist, ist sie meist auch
schneller, weil ein `memcpy` weniger Arbeit ist als eine Base64-Schleife.

Eine Zeile fällt heraus: `dickens` in Profil T ist 4,9 % kleiner und kostet
18 % mehr Kodierzeit. Dort sagt die Stichprobe zu Recht „prüfen" — es gibt
rohe Blöcke —, aber die meisten Blöcke enthalten doch einen Zeilenumbruch und
werden Base64, und für die ist die Prüfung Aufwand. Das ist der eine Fall, in
dem das Format Größe gegen Zeit tauscht, und er ist benannt statt geglättet.

Zum Vergleich, dieselben Dateien: die Segmentfassung kostete auf `dickens`
1137 % beim Kodieren, die Maskenfassung 169 %, diese 100 %.

### 13.4 Kurze Werte

Die 55 kurzen Proben, Profil U, Größe gegen `ceil(4n/3)`, Zeit gegen die
Base64 des Benches (`--example short`):

| Probe | Bytes | Form | Größe | Kodieren, Zeit | Dekodieren, Zeit |
|---|--:|---|--:|--:|--:|
| UUID v4 | 36 | roh | 79 % | **52 %** | **82 %** |
| Session-ID, 40 alnum | 40 | roh | 78 % | **54 %** | **68 %** |
| SHA-512-Digest, hex | 128 | roh | 78 % | **55 %** | **68 %** |
| JWT, drei Segmente | 155 | roh | 79 % | **59 %** | **65 %** |
| Kreditkartennummer | 16 | roh | 82 % | **68 %** | **88 %** |
| Vor- und Nachname | 12 | Base64 | 100 % | **93 %** | 119 % |
| IPv6-Adresse | 28 | Base64 | 100 % | **96 %** | **97 %** |
| SQL-Statement | 118 | Base64 | 100 % | 104 % | **83 %** |
| Logzeile | 93 | Base64 | 100 % | 109 % | **90 %** |
| zufällige 64 Bytes | 64 | Base64 | 100 % | 104 % | **88 %** |
| **alle 55 Proben, als Zeit** | | | | **77 %** | **84 %** |

**Auf kurzen Werten ist Base65t schneller als Base64, in beide Richtungen.**
Der Grund ist die Arbeitsbilanz: Base64 liest ein Byte, schlägt vier Zeichen
nach und schreibt vier — je drei Bytes. Ein Raw-Block liest 48 Bytes, prüft
sie mit sechs Vergleichen je Byte und kopiert sie. Wer weniger schreibt,
schreibt schneller. Die Zeilen, die 100 % Größe haben, kosten höchstens neun
Prozent mehr Zeit, und dekodiert werden sie schneller.

Zum Vergleich, jeweils Zeit: die Segmentfassung lag hier bei 355 % beim
Kodieren, die Maskenfassung bei 98 % beim Kodieren und 123 % beim Dekodieren.

### 13.5 Was lesbar bleibt

Anteil der Bytes, die im Strom stehen wie in der Eingabe
(`--example clear`):

| Datei | Segmentfassung U | Maskenfassung U | **v0.4 U** | **v0.4 T** |
|---|--:|--:|--:|--:|
| Prosa (dickens) | 17 % | 76 % | **0 %** | **24 %** |
| XML | 21 % | 66 % | **0 %** | **45 %** |
| CSS | 54 % | 72 % | **0 %** | **10 %** |
| JSON | 9 % | 15 % | **0 %** | **0 %** |

**Das ist der Preis dieser Fassung, und er ist hoch.** Ein Block wird nur roh,
wenn alle 48 Bytes zulässig sind, und in einem Dokument mit Satzzeichen
kommt das in Profil U nicht vor. Lesbar bleibt, was in Stücken von 48 Bytes
zusammenhängt: Bezeichner, IDs, hexadezimale Werte, in Profil T auch längere
Textabschnitte ohne Anführungszeichen.

Wer lesbaren gemischten Text will, findet ihn nicht hier. Das ist eine
Entscheidung, keine Lücke: die Maskenfassung hat ihn geliefert und dafür das
Dreifache der Base64-Zeit gekostet, und dieses Format lebt davon, dass die
Entscheidung dafür nichts kostet (§0.1, §6).

## 14. Sicherheit

* **Der Dekoder parst überhaupt keine Länge.** Die Segmentfassung stand hier
  hinter Base64: ihr Dekoder las Längen bis 4158 aus dem Strom, die ein
  Angreifer wählen konnte. Hier steht keine Länge im Strom; jede folgt aus
  der Blockform und der Blockgröße. Was bleibt, ist dasselbe wie bei Base64:
  die Gesamtlänge der Eingabe.
* **Rohe Bytes lecken Struktur** — welche Blöcke ganz aus Text bestehen, ist
  im Strom sichtbar, und ihr Inhalt steht im Klartext. Dafür ist
  `encode_base64url` da (§9.3); seine Ausgabe ist gewöhnliches Base64URL.
* **Zwei Auto-Erkennungen sind zwei Parser-Differential-Flächen:** Alphabet
  (§5.2) und Padding (§5.3). Gegenmaßnahmen: Regel A, Regel P,
  `alphabet_seen` / `padding_seen` und `decode_url_strict` (§5.5).
  Differential-Fuzzing ist Pflicht, nicht Kür.
* **Kein Padding-Orakel** — Padding wird nur validiert, nie erzeugt.
* **Malleability** ausgeschlossen auf Blockebene, reduziert auf Alphabet- und
  Padding-Ebene, **nicht** auf Profilebene und nicht dagegen, dass ein
  Fremder einen Raw-Block als Base64-Block umschreibt (§11).
* Dekodierte Ausgabe ist **untrusted binary**, nicht Text.

## 15. Testvektoren

Zwölf Vektoren, jeder als Test in `rust/tests/vectors.rs`. Der maschinell
prüfbare Satz — 173 Einträge über beide Einstiegspunkte und beide Profile —
steht in `docs/vectors.json`. Die Vektoren der früheren Fassungen liegen in
`docs/history/`; ihre *Eingaben* wurden übernommen, ihre Ströme nicht.

### TV1–TV4 — die zwei Formen (Profil U)

| # | Eingabe | Strom | Länge | Base64 wäre |
|---|---------|-------|-------|-------------|
| TV1 | `alice.jones` | `~~alice.jones` | 13 | 15 |
| TV2 | `DE AD BE EF` + `session-eu-central` | `3q2-73Nlc3Npb24tZXUtY2VudHJhbA` | 30 | 30 |
| TV3 | `sub~alice~jones` | `~~sub~alice~jones` | 17 | 20 |
| TV4 | 100 × `a` | `~~` + 48 `a`, `~~` + 48 `a`, `~~aaaa` | 106 | 134 |

**Zu TV2.** Vier der 22 Bytes sind nicht zulässig, also ist der Block Base64,
und der Strom ist byteweise `encode_base64url`. Die Segmentfassung schrieb
diese Eingabe in 26 Zeichen; das ist der Preis für einen Encoder, der eine
Vergleichsoperation ist.

**Zu TV3.** `~` in einem Raw-Block braucht nichts, weil die Blocklänge
feststeht. `hello~Alice` wird `~~hello~Alice`.

### TV5 — ein Byte entscheidet den Block

```
Eingabe:  the quick brown fox jumps over the lazy dog. again      (50 Bytes)
Profil U: dGhlIHF1aWNrIGJyb3duIGZveCBqdW1wcyBvdmVyIHRoZSBsYXp5IGRvZy4gYWdhaW4
          67 Zeichen, byteweise Base64URL — das Leerzeichen ist in U nicht zulässig
Profil T: ~~the quick brown fox jumps over the lazy dog. aga  aW4
          53 Zeichen: der erste Block roh, die zwei Rest-Bytes als Base64
```

### TV5b — die reservierte Form

`~AAAAAAAA`, `~7abc`, `~_` → `E_RESERVED`. `~=`, `~ a` → `E_CHARSET`. Der
Unterschied ist normativ: das erste ist ein Strom einer Fassung, die dieser
Dekoder nicht kennt, das zweite ist kaputt.

### TV6 — Abwärtskompatibilität

| Strom | Bytes | `alphabet_seen` | `padding_seen` |
|-------|-------|-----------------|----------------|
| `PDw_Pz8-Pg` | `<<???>>` | url | false |
| `PDw/Pz8+Pg` | `<<???>>` | classic | false |
| `YWxpY2Uuam9uZXM` | `alice.jones` | none | false |
| `YWxpY2U=` | `alice` | none | true |

Ein Base64-Strom beliebiger Länge liest sich in Blöcken von 64 Zeichen, und
das ist unsichtbar, weil Base64-Blöcke kacheln.

### TV7 — Alphabet-Konsistenz

`PDw_Pz8+Pg` und `PDw/Pz8-Pg` → `E_MIXED_ALPHABET`. Die Regel gilt über
Raw-Blöcke hinweg: ein URL-Base64-Block, ein Raw-Block, ein Classic-Block →
`E_MIXED_ALPHABET`. Rohe Bytes zählen nicht mit: `~~a+b/c-d_e` in Profil T
ist `alphabet_seen = none`.

### TV8 — was auf ein `~` folgen darf

`~` → `E_TRAILING_TILDE`. `~~` → leerer Raw-Block, gültig, null Bytes.
`~A` → `E_RESERVED`. `~=` → `E_CHARSET`. `YW~x` → `E_CHARSET`, weil ein `~`
mitten in einem Base64-Block steht.

### TV9–TV10 — Padding

```
YWxpY2U=     -> "alice",  padding_seen
YWxpY2Uu     -> "alice.", kein Padding
YWxp==       -> E_PADDING
YWxpY2U==    -> E_PADDING
```

Ein `=` am Ende des 64. Zeichens eines Base64-Blocks, dem ein weiterer Block
folgt, ist `E_CHARSET`. TV10, Profil T: `~~a=b=` ist ein Raw-Tail und trägt
vier Bytes, davon zwei `=`; in Profil U ist es `E_PROFILE`.

### TV11 — Fehlerfälle

| Strom | Code |
|-------|------|
| `abcde` | `E_ALIGN` |
| `~` | `E_TRAILING_TILDE` |
| `~Aabc` | `E_RESERVED` |
| `~~a b` | `E_PROFILE` (Profil U) |
| `YWxp==` | `E_PADDING` |
| `YWxpY2V` | `E_NONZERO_TAIL` |
| `YW~x` | `E_CHARSET` |
| `PDw_Pz8+Pg` | `E_MIXED_ALPHABET` |

### TV12 — der Tail

Ein letzter Block folgt §9.1: roh ab vier Bytes, Base64 darunter,
Gleichstand an die rohe Form. Nach einem vollen Raw-Block:

| Tail | Strom des Tails |
|---|---|
| — | — |
| `a` | `YQ` |
| `abc` | `YWJj` |
| `abcd` | `~~abcd` |
| `a b` | `YSBi` |
| `a bcd` | `YSBiY2Q` |

## 16. Konformitätsnachweise

Eine Implementierung gilt als konform, wenn sie die drei folgenden
Eigenschaften belegt:

1. **`decode(encode(x)) == x`** für beide Profile, über einen Fuzzing-Korpus.
2. **`decode(base64(x)) == x`** und **`decode(base64url(x)) == x`** für alle
   kanonischen Eingaben, gepaddet und ungepaddet — per Differential-Fuzzing gegen die
   Standard-Base64-Bibliothek der jeweiligen Sprache. Erwartete Abweichungen
   (`E_NONZERO_TAIL`, §1.1) gehören als solche in den Korpus.
3. **`encode(x, profil)` byte-identisch über zwei unabhängige
   Implementierungen**, über den gesamten Vektorsatz.
   **Erbracht, mit einer benannten Lücke.** `rust/` und
   `conformance/reference.py`, die zweite aus diesem Dokument geschrieben und
   ohne eine Zeile gemeinsamen Code. Sie stimmen über alle 308 Vektorpaare
   überein, über fünfzehn Fehlerfälle, und über eine 262923-Byte-Eingabe
   Zeichen für Zeichen (`conformance/test_large.py`). Die Lücke: derselbe
   Autor.

Ergänzende Arbeiten, nicht normativ:

4. Messen (§12, §13): Korpusdichte und Durchsatz über binary2textbench —
   **erbracht**, die Zahlen stehen dort.
5. Container-Test mit echten Parsern — **erledigt für Pythons Parser**,
   `conformance/test_containers.py`. Profil U geht durch URL, Cookie, JSON,
   Dateiname und Logzeile unverändert; Profil T braucht in einer URL
   Prozent-Encoding und enthält das Leerzeichen.
6. API-Form je Zielsprache: `encode` / `decode` analog zum dortigen `base64`;
   zusätzlich `decode_url_strict` und `encode_base64url`, und sonst nichts.
   Rust liegt bei; `python/` ist ein PyO3-Binding darüber. Ein Binding ist
   ausdrücklich **keine** zweite Implementierung im Sinne von Nachweis 3.
7. Vektorsatz: `docs/vectors.json` führt 173 Vektoren. Sie decken die
   Blockgrenze bei 48, die Tails von 1 bis 6 Bytes, an denen die rohe Form
   einsetzt, und Blöcke ab, denen ein einziges Byte zur rohen Form fehlt.

## 17. Erweiterungskandidaten (nicht Teil von v0.4)

1. **Eine dritte Blockform**, die einen gemischten Block teilweise im
   Klartext trägt. `~` gefolgt von einem Alphabetzeichen ist dafür reserviert
   (§6), und `docs/history/` beschreibt die Fassung, die es einen Tag lang
   gab: eine Maske mit einem Bit je Byte. Sie hat gemischten Text lesbar
   gemacht und das Dreifache der Base64-Zeit gekostet. Wer sie wieder
   einführt, muss zeigen, dass sie ohne diesen Preis geht — eine
   vektorisierte Compress-Operation wäre der Weg —, und braucht eine neue
   Versionsnummer.
2. **Wahlfreier Zugriff.** Blockgrenzen liegen an festen Eingabe-Offsets,
   aber an variablen Ausgabe-Offsets. Ein Index der Blockanfänge, außerhalb
   des Stroms, gibt O(1)-Zugriff.
3. **Profil-Aushandlung.** Aus dem Strom prinzipiell nicht ableitbar (§7.2).
4. **Eine andere Blockgröße.** 48 ist begründet (§4), nicht bewiesen. Eine
   größere Blockgröße drückt die rohe Form gegen 75 % und lässt gleichzeitig
   häufiger einen ganzen Block nach Base64 kippen; wo das Optimum liegt, ist
   eine Messfrage. Wer die Zahl ändert, ändert die Versionsnummer.
5. **Vektorbreite zur Laufzeit wählen.** Kein Formatthema und keine Lücke im
   Code: die Prüfung aus §13.1 vektorisiert bereits, auf dem Basis-Ziel
   `x86-64` mit 16 Bytes je Operation. Wer mit `-C target-cpu=native` baut,
   bekommt 32 oder 64 und halbiert den Aufschlag beim Kodieren — heute, ohne
   `unsafe` und ohne Codeänderung. Was fehlt, ist dasselbe **ohne Bauflag**,
   also eine Erkennung zur Laufzeit mit mehreren Varianten derselben
   Funktion. Dafür gibt es zwei Wege, und beide sind heute versperrt:
   `#[target_feature]` verlangt `unsafe`, was §14 ausschließt, und
   `std::simd` ist nicht stabil (geprüft auf rustc 1.94.1, Tracking-Issue
   rust-lang/rust#86656). Sobald `std::simd` stabil ist, sind es wenige
   Zeilen, die kein Byte der Ausgabe bewegen.
