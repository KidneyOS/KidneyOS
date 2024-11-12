break _start
break thread_control_block.rs:206
  commands
    silent
    print phys_start
    print new_phys_addr
    print len
    cont
  end
break thread_control_block.rs:213
  commands
    silent
    printf "Copy completed!\n"
    cont
  end
