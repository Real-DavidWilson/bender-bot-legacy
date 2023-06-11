## Bender Bot

Bender é um bot de código aberto feito para gerenciar servidores do Discord. O bot ainda está em processo de desenvolvimento, portanto seu uso é instável.

### Instruções

Para executar o bot instale as dependências e configure o ambiente com as instruções a seguir.

#### Instale o Rust

https://www.rust-lang.org/pt-BR/tools/install

#### Instale o FFmpeg Cli

https://ffmpeg.org/download.html

#### Instale o Youtube-DL Cli

https://ytdl-org.github.io/youtube-dl/download.html

#### Configure o ambiente

Defina as variáveis de ambiente a partir do [arquivo de exemplo](.env.example).

#### Compilando o bot

Após todas as dependências instaladas e configurações concluidas, você pode por fim compilar o bot executando o seguinte comando:

```bash
cargo build --release 
```