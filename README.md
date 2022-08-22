# Server
 A server in Rust, based off the CS3214 Project 4 Server specification

# Performance 
Meassured by server_bench.py

| Test       | Connections | Base                        | Rust                         |
|------------|-------------|-----------------------------|------------------------------|
| login40    | 40          | 324140.00 r/s or 32.15MB/s  | 472774.73 r/s or 46.89MB/s (+45%)  |
| login500   | 448         | 698437.95 r/s or 69.27MB/s  | 1565539.89 r/s or 155.27MB/s (+124%) |
| login10k   | 9984        | 601785.11 r/s or 59.70MB/s  | 1084089.59 r/s or 107.52MB/s + (80%) |
| wwwcsvt100 | 64          | 16992.95 r/s or 1.09GB/s    | 16890.93 r/s or 1.09GB/s (-0.6%)    |
| doom100    | 40          | 510.61 r/s or 1.10GB        | 510.39 r/s or 1.10GB/s   (-0.04%)    |