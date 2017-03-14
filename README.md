# BACKFORTH

Backforth is a concatenative language created by someone who considers such languages categorically unreadable. It is fair to call it a failed experiment. The name "Backforth" alludes to the unconventional parsing model, whereby evaluation begins at the end of the line and proceeds to the left.

It looks like this:

```backforth
print = {
    echo
}

x = + * 3 2 1

print x # Prints "7"
```

The first version was written in Haxe some time in 2013 or 2014. It never worked right and the source code was tragically lost, but I've reimplemented it in Rust based on my foggy memories.
