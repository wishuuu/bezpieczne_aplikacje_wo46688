## Wiszowaty Oskar 46688

Basic Auth:
Metoda autoryzacji polegająca na przesyłaniu loginu i hasła w nagłówku HTTP. Ciąg znaków login:hasło jest kodowany w base64 i przesyłany w nagłówku Authorization (Basic).

OAuth:
Metoda autoryzacji oparta o tokeny autoryzacyjne. Token może zostać wygenerowany przez zewnętrzny serwis autoryzacyjny. Otrzymany token przesyłany jest w nagłówku Authorization (Bearer).

Zalety Basic Auth:

- Prostota implementacji
- Wsparcie w większości języków programowania
- Szybkość działania
- Brak konieczności przechowywania tokenów

Wady Basic Auth:

- Przechwycenie zapytania HTTP pozwala na odczytanie loginu i hasła
- Brak możliwości wybaszenia tokenu

Zalety OAuth:

- Wyższe bezpieczeństwo
- Możliwość wydłużenia ważności tokenu
- Możliwość odwołania tokenu

Wady OAuth:

- Skomplikowana implementacja

Sytuacje kiedy warto użyć Basic Auth:

- Proste aplikacje
- Aplikacje działające w zamkniętej sieci
- Aplikacje, w których bezpieczeństwo nie jest priorytetem

Sytuacje kiedy warto użyć OAuth:

- Aplikacje, w których bezpieczeństwo jest priorytetem
- Aplikacje, które ze względów biznesowych wymagają autoryzacji przez zewnętrzny serwis
- Publicznie dostępne aplikacje
