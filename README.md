[![Build Status](https://travis-ci.org/msehnout/lunch-bot.svg?branch=master)](https://travis-ci.org/msehnout/lunch-bot)

# IRC lunch bot

Features: 
- [x] create groups of users
- [x] translate names from base to augmented (e.g. user -> user|wfh)
- [x] propose places and times, list proposals
- [x] delete old proposals automatically
- [x] periodically safe global state for recovery purposes

Dev TODO:
- [ ] improve logging
- [ ] improve inline docs

Getting started
```
1. Create a group (use names, that are not augmented with |wfh or |ooo etc.)
lb group add GROUPNAME USER1,USER2,USER3

2. Propose a place
lb propose RESTAURANT 12:00 to GROUPNAME

3. See available proposals
lb list proposals
```

Usage:
```
  lb propose <place>[ at] <time>[ to <group>][ meet <place> <time>]
  lb list (groups|proposals)
  lb group (add <group-name> <comma-separated-list-of-users>|remove <group-name>)
  lb add <user> to <group>
```
