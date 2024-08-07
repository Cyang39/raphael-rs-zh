# 汉化说明

:link: [raphael.hqy.life](https://raphael.hqy.life/)

1. 中文字体使用 [MiSans](https://hyperos.mi.com/font/zh/)；
2. 原应用已经支持 7.0，技能参数按照国服调整回 6.x，删除的[注视制作](https://ff14.huijiwiki.com/wiki/Action:100238)和[注视加工](https://ff14.huijiwiki.com/wiki/Action:100246)未添加回；
3. 物品数据来源 [ffxiv-datamining-cn](https://github.com/thewakingsands/ffxiv-datamining-cn)，国服没有的物品显示日文。

<hr>

# Raphael XIV [<img src="https://img.shields.io/discord/1244140502643904522?logo=discord&logoColor=white"/>](https://discord.com/invite/m2aCy3y8he)

:link: [www.raphael-xiv.com](https://www.raphael-xiv.com/)

Raphael is a crafting rotation solver for the online game Final Fantasy XIV.
* Produces optimal solutions. Achieving higher quality than the solver is impossible.
* Short solve time (5-60 seconds) and reasonable memory usage (300-1000 MB).

## How does it work?

* Short answer: [A* search](https://en.wikipedia.org/wiki/A*_search_algorithm) + [Pareto optimization](https://en.wikipedia.org/wiki/Multi-objective_optimization) + [Dynamic programming](https://en.wikipedia.org/wiki/Dynamic_programming).
* Long answer: coming soon<sup>tm</sup>

## Building from source

The [Rust](https://www.rust-lang.org/) toolchain is required to build the solver.

### Native

To build and run the application:

```
cargo run --release
```

### Web (wasm)

[Trunk](https://trunkrs.dev/) is required to bundle and host the website and can be installed via the Rust toolchain:

```
cargo install --locked trunk
```

To build and host the application locally:

```
export RANDOM_SUFFIX=""
export RUSTFLAGS="--cfg=web_sys_unstable_apis"
trunk serve --release --dist docs
```
