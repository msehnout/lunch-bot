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

Usage:
```
  lb propose <place>[ at] <time> [to <group>]
  lb list (groups|proposals)
  lb group (add <group-name> <comma-separated-list-of-users>|remove <group-name>)
  lb add <user> to <group>
```
