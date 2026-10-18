# MPL 2.0 对应版本源码获取说明

SciCanvas Windows x64 使用 MPL 2.0 的 resvg、usvg、usvg-parser、usvg-text-layout、usvg-tree，当前版本均为 0.37.0。这些部分及其源码继续适用 MPL 2.0，接收者的 MPL 权利保持原样。

完整的已发布源码包随本发行包提供在 `licenses/mpl-sources/*.crate`，可以离线获取。`.crate` 是 tar.gz，使用 7-Zip 或 `tar -xf 文件名.crate` 解包后即可查看、修改及重新构建原库；库源码包含 Cargo.toml 和原许可证。无需登录 SciCanvas 的私密仓库。

| 组件 | 随包源码 | 官方备用下载 |
| --- | --- | --- |
| resvg 0.37.0 | [resvg-0.37.0.crate](licenses/mpl-sources/resvg-0.37.0.crate) | [crates.io 原包](https://static.crates.io/crates/resvg/resvg-0.37.0.crate) |
| usvg 0.37.0 | [usvg-0.37.0.crate](licenses/mpl-sources/usvg-0.37.0.crate) | [crates.io 原包](https://static.crates.io/crates/usvg/usvg-0.37.0.crate) |
| usvg-parser 0.37.0 | [usvg-parser-0.37.0.crate](licenses/mpl-sources/usvg-parser-0.37.0.crate) | [crates.io 原包](https://static.crates.io/crates/usvg-parser/usvg-parser-0.37.0.crate) |
| usvg-text-layout 0.37.0 | [usvg-text-layout-0.37.0.crate](licenses/mpl-sources/usvg-text-layout-0.37.0.crate) | [crates.io 原包](https://static.crates.io/crates/usvg-text-layout/usvg-text-layout-0.37.0.crate) |
| usvg-tree 0.37.0 | [usvg-tree-0.37.0.crate](licenses/mpl-sources/usvg-tree-0.37.0.crate) | [crates.io 原包](https://static.crates.io/crates/usvg-tree/usvg-tree-0.37.0.crate) |

每包的 SHA-256、版本、官方 URL 记录在 [第三方索引](licenses/THIRD_PARTY_INDEX.json) 的 `mpl` 项，与 Cargo.lock 的 checksum 一致。打包工具会验证归档校验和及本地 MPL 源文件的完整性；发现本地修改时停止，要求提供包含修改的对应源码。启用 resvg 的 text/system-fonts 属于构建特性选择，原包源码保持不变。

MPL 完整文本见 [resvg 原始许可证](licenses/third-party/resvg-0.37.0/LICENSE.txt)。来源义务参见 [Mozilla 官方 FAQ 第 8–10 问](https://www.mozilla.org/en-US/MPL/2.0/FAQ/)。第三方材料的许可和版权声明优先适用于对应文件，SciCanvas 的 MIT/Apache 选择不重新许可这些材料。

维护者发布时应将此说明、第三方声明、许可证和这些源码包与 exe 一起分发；依赖版本或源码有变化时先重新生成并核对。发行包 `BUILD_INFO.json` 记录实际构建版本与二进制校验和。
