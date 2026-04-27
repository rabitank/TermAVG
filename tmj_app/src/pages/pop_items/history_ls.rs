use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};

/// 对话记录示例
#[derive(Debug, Clone)]
pub struct DialogueRecord {
    pub id: usize,
    pub speaker: String,
    pub content: String,
    // 可根据需要扩展其他字段
}

/// 历史记录列表，有最大容量，push 时自动弹出最旧的记录
pub struct HistoryLs {
    max_capacity: usize,
    records: VecDeque<DialogueRecord>,
}

impl HistoryLs {
    /// 创建一个新的 HistoryLs 实例
    pub fn new(max_capacity: usize) -> Self {
        HistoryLs {
            max_capacity,
            records: VecDeque::with_capacity(max_capacity),
        }
    }

    /// 添加一条记录，若超出容量上限，自动移除最旧的一条
    pub fn push(&mut self, record: DialogueRecord) {
        if self.records.len() >= self.max_capacity {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// 获取当前所有记录（不可变引用）
    pub fn records(&self) -> &VecDeque<DialogueRecord> {
        &self.records
    }

    /// 获取当前记录数量
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// 全局历史记录（线程安全）
/// 容量上限设为 100，可根据需求调整
pub static HISTORY_LS: LazyLock<Mutex<HistoryLs>> = LazyLock::new(|| {
    Mutex::new(HistoryLs::new(100))
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_evict() {
        let mut history = HistoryLs::new(3);
        history.push(DialogueRecord { id: 1, speaker: "Alice".into(), content: "Hi".into() });
        history.push(DialogueRecord { id: 2, speaker: "Bob".into(), content: "Hello".into() });
        history.push(DialogueRecord { id: 3, speaker: "Alice".into(), content: "How are you?".into() });
        history.push(DialogueRecord { id: 4, speaker: "Bob".into(), content: "Fine".into() });

        assert_eq!(history.len(), 3);
        let records: Vec<_> = history.records().iter().map(|r| r.id).collect();
        assert_eq!(records, vec![2, 3, 4]); // 最早的 id=1 已被移除
    }

    #[test]
    fn test_global_history() {
        let mut history = HISTORY_LS.lock().unwrap();
        history.push(DialogueRecord { id: 10, speaker: "System".into(), content: "Start".into() });
        assert_eq!(history.len(), 1);
    }
}
