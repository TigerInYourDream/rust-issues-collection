## 设计抉择问题

在 /Users/zhaoyue/workspace/pad-work/makepad-project/robrix 路径下 存在一个问题

有一个共用的TOKIO_RUNTIME 包装了一个tokio runtime. 需要在运行的时候使用 一般有两处是主要的

1. 是tsp需要
2. 再退出的时候需要借助 tokio_runtime来退出

使用Arc包装可以放在在tsp需要使用runtime的时候生成一份但是Arc不具备主动（手动）处理所有引用的能力使用Arc包装的runtime无法保证在退出的时候及时关闭runtime

使用Mutex包装的runtime可以保证是唯一引用，但是在tsp使用的时候就笔比较复杂
