import mpmath
mpmath.mp.dps = 300000
x = mpmath.pi + mpmath.e
y = mpmath.exp(x) * mpmath.sqrt(x) / mpmath.log(x + 1)
print(int(y) % 1000)
