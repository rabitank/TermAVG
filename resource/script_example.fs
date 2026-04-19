#
bgm.set "resource/bgm1.mp3"
set bgimg_path "resource/bg_classroom.png"
p1 = character "resource/fc.chr"
p2 = character "resource/bxy.chr"
character_ls.set_characters p1 p2
p1.say "我觉得我们得把那个 isDataReady 标志位改成 isReadyData，这样语义更自然"

#
set p2.face smile
p2.say "但是 isReadyData 少了一个‘a’，你说的是 isReadyData 还是 isDataReady？"

#
p1.say "就是那个……算了，我决定用 bReady，前面加个匈牙利命名法的 b，表示布尔型"

#
p2.say "好，那 bReady 是 true 的时候代表什么？"

#
once p1.face smile
p1.say "代表它 not false"

#
p2.say "那 not false 不就是 true 吗？"

#
p1.say "对，所以 bReady 为 true 的时候，它其实是 !false，我们得小心别搞混"

#
p2.say "那 !false 和 true 有什么区别？"

#
p1.say "区别在于……!false 多了一个感叹号，性能上会慢一点点"

#
once p2.face cry
p2.say "那我们把 bReady 默认值设成 false，然后什么时候改成 !false？"

#
p1.say "等 isDataReady 变成 true 的时候"

#
p2.say "但 isDataReady 已经改名叫 bReady 了啊。"

#
p1.say "哦对，那我们需要一个中间变量 tempReady 来过渡一下"

#
p2.say "那 tempReady 用完了要不要删掉？"

#
p1.say "不用，我们把它注释掉，下次万一用得上"

#
p2.say "行，那我现在写代码了。这个变量名用 _bReadyTemp_2_final_FINAL 没问题吧？"

#
p1.say "完美，记得加下划线开头，避免和标准库冲突"

#
set p1.face however
p2.say "可我们连标准库都没引用。"

#
p1.say "那更好，更安全了"

#
p2.say "话说……我们刚才到底要解决什么问题来着？"

#
p1.say "我也不知道，但我已经改完变量名了，现在代码可读性提高了 100%"

