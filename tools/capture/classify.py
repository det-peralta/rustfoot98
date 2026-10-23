import sys, numpy as np
from PIL import Image
a=np.array(Image.open(sys.argv[1]).convert('RGB')).astype(int)
def px(x,y): return tuple(a[y,x])
Y=(248,252,0); W=(248,252,248); N=(0,0,128); K=(0,0,0); S=(192,192,192)
kind='?'
if px(150,200)==Y and px(600,300)==Y: kind='auction'
elif px(300,91)==W and px(400,72) in (N,S): kind='main'
elif px(400,5)==N or px(400,10)==N:
    kind='roundview'
    if px(300,200)==K and px(400,160)==K: kind='penalty'
    elif px(300,150)==S or px(250,300)==S: kind='halftime'
elif px(400,5)==S:
    if px(300,300)==K and px(400,150)==K: kind='penalty'
    elif px(300,300)==S or px(450,200)==S: kind='halftime'
    elif px(300,91)==W: kind='main_inactive'
    else: kind='dialog_over_roundview'
print(kind)
