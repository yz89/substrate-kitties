### homework 4

#### 转移猫的设计

修改链上数据存储结构，如下：
```
        /// Stores all the kitties, key is the kitty id / index
        pub Kitties get(kitty): map T::KittyIndex => Option<Kitty>;
        /// Stores the total number of kitties. i.e. the next kitty index
        pub KittiesCount get(kitties_count): T::KittyIndex;
        /// Get kitty owner by kitty index
        pub OwnedKitties get(owned_kitties): map T::KittyIndex => T::AccountId;
```

将存储 ownership 的字典修改为 `map T::KittyIndex => T::AccountId`， 这样每次转移小猫只需要做一次字典的 update 操作即可。

#### transfer 功能流程

1. 检查要转移的小猫是否存在
2. 检查 sender 是否拥有这只小猫
3. 更新 OwnerKitties 字典

实现代码：

```
    fn do_transfer(
        sender: T::AccountId,
        recipient: T::AccountId,
        kitty_id: T::KittyIndex,
    ) -> Result {
        // Check if the kitty exsit
        let transfer_kitty = Self::kitty(kitty_id);
        ensure!(transfer_kitty.is_some(), "Invalid transfer kitty");

        // Check if the sender own this kitty
        ensure!(Self::owned_kitties(kitty_id) == sender, "Sender must own the transfer kitty");

        // Store the ownership information
        <OwnedKitties<T>>::insert(kitty_id, recipient);

        Ok(())
    }
```