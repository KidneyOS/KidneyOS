break _start
break src/interrupts/mod.rs:50
  commands
    silent
    where
    cont
  end
break src/interrupts/mod.rs:43
  commands
    silent
    printf "Interrupts enabled again!\n"
    cont
  end
break sleep.rs:129
break ata_core.rs:80
