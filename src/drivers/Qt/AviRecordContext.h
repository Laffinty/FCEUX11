// AviRecordContext.h
//

#pragma once

struct AviRecordContext
{
	bool sessionActive = false;
	bool audioEnabled = false;
	int  videoDriverIndex = 0;
	int  videoFormatIndex = 0;
};
