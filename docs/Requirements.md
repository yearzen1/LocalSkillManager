## 背景
ai时代，开源项目，不仅要开源代码，还应该开源skill和agentsmd。
skill的目录还要所有agent都直接直接使用。
目前最广的规范是.agents/skills/。
所以我要实现一个skillmanager，能创建skill到全局和项目目录，也能复制全局skill到项目skill

## 功能要求
1. 创建skill安装到全局目录和项目目录，目录格式采用.agents格式
2. 能将全局skill复制到项目目录中，项目目录格式也是.agents格式
## 产品形态和运行平台
1. cli工具  
2. 能运行在windows和linux和macos
## 命令介绍

### 完整版
```plaintext
1. lsm add <xxx>.md global
2. lsm add <xxx>.md project
3. lsm list global
4. lsm list project
5. lsm copy <xxx>
```

### 缩减版
```plaintext
1.-a = add
2.-g = global
3.-p = project
4.-l = list
5.-c = copy
```