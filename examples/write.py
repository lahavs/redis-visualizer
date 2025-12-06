#!/usr/bin/env python3

import redis

r = redis.Redis(host='localhost', port=6379, db=0)

for i in range(1000):
    key = f'my_key{i}'
    data = {f'key{i}_{j}': f'val{i}_{j}' for j in range(100) }
    r.hset(key, mapping=data)

long_key = 'l' + 'o'*40 + 'ng key'
long_data = {'l' + 'o'*40 + 'ng key': 'l' + 'o'*40 + 'ng value'}
r.hset(long_key, mapping=long_data)
