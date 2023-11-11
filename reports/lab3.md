# 实验3
- 增加spawn系统调用,先获取etl_data的数据和pid，在组合成一个TaskControlBlock,并将任务加入任务队列后，返回pid
- 实现stride调度，更改taskManager的fetch方法，选择stride值最小的任务返回
- 在suspend_current_and_run_next将上次运行的任务的stride增加pass量
- 将taskManager中的计算系统调用，mmap、munmap等方法移动到processor.rs下

## 问答作业
- 1.可以使用写时拷贝,即fork之后的子进程和父进程共用同样的数据当这些页发生修改时，将其拷贝一份做为当前进程的独有数据
- 3. 父进程先还是子进程先结果都没什么变化，只是输出顺序不一样， 父进程输出 0 1 2 子进程 3 4 
- 4. 36
- stride会有溢出的情况导致本来是p1执行实际是p2执行， 因为优先级>=2 pass最大是BigStride / 2，最开始所有进程的stride都相等，每次取最小的stride增加pass
所以导致STRIDE_MAX – STRIDE_MIN永远不会超过最大pass,因为存在这种情况的话，被增加pass的进程就不可能是最小stride


## 实验难点
- 本节课是进程调度实现stride算法较难，需要考虑最大BigStride 的值，因为这个值要能被整除才比较公平，否则会因为小数点丢失导致不太公平
- 难度比实验简单些，好理解些。因为这个调度和任务切换的控制流转换的方法在第三章已经学习过，就没有那么难了
- 望深的去探索的话，比如调度算法估计还是会挺难的