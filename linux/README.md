# Reset Explode — Linux em Rust

Interface GNOME em GTK 4/libadwaita. Esta versão não inclui todas as funcionalidades do Reset Explode original. Mostra informações de sistema, memória, sensores de temperatura, GPU, armazenamento, rede, portas seriais USB e bateria, quando disponíveis no Linux, além de listar jogos encontrados nas bibliotecas Steam.

A memória usada é calculada como `MemTotal - MemAvailable`, em GiB e porcentagem. Núcleos físicos são contados pela topologia de CPUs online; threads online são distinguidas das disponíveis ao aplicativo quando diferem. A média de temperatura usa sensores Intel coretemp por núcleo, quando disponíveis. Os discos exibem caminho, fabricante e capacidade em GiB quando informados pelo sistema; quando necessário, o fabricante é identificado por prefixos conhecidos do modelo. Sem identificação confiável, é exibido como não informado.

A GPU é identificada por meio da base PCI da distribuição (`pci.ids` no Debian/Ubuntu e `hwdata` no Fedora), quando disponível. O botão Exportar HTML salva as leituras e o último benchmark da sessão. Também é possível usar `reset-explode-linux --report` ou `reset-explode-linux --export-html arquivo.html` no terminal.

A página Benchmark inclui uma porta inicial do renderizador de CPU do original, com a cena fixa de 960 × 540 pixels, 1.024 amostras por pixel, uma passada, modo single ou multi-core, progresso e cancelamento. Uma janela mostra a cena sendo formada; fechá-la durante o teste cancela a execução. A prévia gráfica pode influenciar a pontuação. A fórmula é amostras por segundo divididas por 10.000. Seis pixels e a contagem de amostras foram comparados com o renderizador .NET original; uma medição da nota Rust ficou cerca de 7% acima da versão .NET na mesma máquina. A equivalência de desempenho entre sistemas ainda não foi validada. Também há um teste rápido de CPU independente. Execute a renderização no terminal com `--render-single` ou `--render-benchmark`.

A interface oferece modo claro e modo escuro inspirado no Night Owl. A escolha é salva em `$XDG_CONFIG_HOME/reset-explode/theme` (ou `~/.config/reset-explode/theme`). As leituras dependem do hardware, drivers e permissões; a versão original está preservada fora desta pasta.

Para compilar, instale Rust, GTK 4, libadwaita e seus arquivos de desenvolvimento e execute `cargo build --release`. Para testar, execute `cargo test`. No Debian/Ubuntu, os requisitos de compilação incluem `libgtk-4-dev`, `libadwaita-1-dev` e `build-essential`. Os pacotes são x86-64 e foram compilados no Ubuntu 24.04; podem exigir bibliotecas pelo menos tão recentes quanto as desse sistema.

Esta versão é distribuída sob a GNU General Public License v3 (GPLv3); veja LICENSE. Esta versão não é uma versão oficial dos autores originais.
