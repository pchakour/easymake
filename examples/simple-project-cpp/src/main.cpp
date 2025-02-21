#include "math.hpp"
#include <iostream>

Fraction half = create_half();

int main()
{ 
    Fraction third { 1, 3 };

    Fraction res = mult(half, third);
    std::cout << res.num << std::endl;
}