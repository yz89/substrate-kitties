### homework 5

#### 交易所设计

新增 exchange 模块，链上数据存储结构，如下：
```
        pub ExchangeKitties get(exchange_kitty): map (T::AccountId, T::KittyIndex) => u32;
```
卖家与猫的 id 作为 key，价格作为 value

#### exchange 功能流程

1. 查询猫的价格
2. 按照猫的价格，转账到卖家账户
3. 删除卖家与小猫的所有关系
4. 新增买家与小猫的所有关系

伪代码：
```
    fn do_exchange(
        sender: T::AccountId,
        recipient: T::AccountId,
        kitty_id: T::KittyIndex,
    ) -> Result {
        // 查询猫的价格
        let price = Self::ExchangeKitties(&(sender, kitty_id))

        // 转账
        send_transaction();

        // 删除卖家与猫的所属关系
        <OwnedKitties<T>>::remove(&sender, kitty_id);

        // 新增买家与猫的所属关系
        <OwnedKitties<T>>::append(&recipient, kitty_id);

        Ok(())
    }
```