import re
import json

raw_text = """
1. 3 PegSharry Mann, Mista Baaz
2. Angreji BeatYo Yo Honey Singh, Gippy Grewal
3. Bapu ZimidarJassi Gill
4. Born To ShineDiljit Dosanjh
5. Cute MundaSharry Mann
6. Daru Peeke Daru PikeKaka Bhainia Wala
7. Dhol Jageero DaMaster Saleem, Panjabi MC
8. Diamond GurnamGurnam Bhullar
9. Dope ShopeYo Yo Honey Singh, Deep Money
10. G.O.A.T.Diljit Dosanjh
11. Gallan Goriyan MannHarbhajan Mann
12. Gora RangGurnam Bhullar
13. Gud Naal Ishq MithaNavraj Hans, Harshdeep Kaur, Rochak Kohli
14. Gulabi PaggDiljit Dosanjh
15. Handsome JattaJordan Sandhu
16. Ik TeraManinder Buttar
17. Ishq Tera Tadpave (Oh Ho Ho Ho)Sukhbir
18. Jatt ZimidaarGurnam Bhullar
19. Jee Ni LagdaKaran Aujla
20. Jind MahiDiljit Dosanjh
21. KadarMankirt Aulakh
22. Kangna Tera NiDr Zeus
23. Laung GawachaBally Sagoo, Rama
24. LehangaJass Manak
25. London ThumakdaLabh Janjua, Sonu Kakkar, Neha Kakkar
26. MakhnaTanishk Bagchi, Yasser Desai, Asees Kaur
27. Mirza Panjabi MCPanjabi MC
28. Morni BankeGuru Randhawa, Neha Kakkar, Tanishk Bagchi
29. Muchh Phut GabruJordan Sandhu
30. Mundian To Bach KePanjabi MC
31. Nachan Ton PehlanYuvraj Hans
32. Nachde Ne SaareJasleen Royal, Harshdeep Kaur, Siddharth Mahadevan
33. Pani Di GalManinder Buttar, Asees Kaur
34. Phone Maar DiGurnam Bhullar
35. Rim vs JhanjarKaran Aujla
36. Sadi Gali Aaya KaroRDB
37. SakhiyaanManinder Buttar
38. Sauda Khara KharaDiljit Dosanjh, Sukhbir, Dhvani Bhanushali
39. Sawan Mein Lag Gayi AagMika Singh
40. Shopping JassJass Manak
41. Suit PunjabiJass Manak
42. Suit SuitGuru Randhawa, Arjun
43. Tera Mera ViahJass Manak
44. Trending NakhraAmrit Maan
45. Tunak Tunak TunDaler Mehndi
46. VeervaarDiljit Dosanjh
47. ViahJass Manak
1. 5 TaaraDiljit Dosanjh
2. All BlackSukh-E Muzical Doctorz, Raftaar
3. All Night APAP Dhillon
4. Bachke BachkeKaran Aujla, Y Shorty
5. BandanaShubh
6. Chaar Botal VodkaYo Yo Honey Singh
7. Chitta KurtaKaran Aujla
8. Chu Gon DoKaran Aujla
9. Daru BadnaamKamal Kahlon, Param Singh
10. DefaulterJordan Sandhu
11. Dil Le Gayee KudiJasbir Jassi
12. DiorShubh
13. Don't Be Shy Dr ZeusDr Zeus, Rouge
14. Don't LookKaran Aujla
15. DroptopAP Dhillon, Gurinder Gill
16. FamousSidhu Moosewala
17. Gal Ban GayiMeet Bros, Sukhbir, Neha Kakkar, Yo Yo Honey Singh
18. Gall Da KaraMankirt Aulakh
19. Gur Naalo Ishq MithaMalkit Singh, Bally Sagoo
20. High EndDiljit Dosanjh
21. Hornn BlowHarrdy Sandhu
22. IceShubh
23. JaguarSukh-E Muzical Doctorz, Bohemia
24. Jatt Fire KardaDiljit Dosanjh
25. JogiPanjabi MC
26. Jugni JiDr Zeus, Kanika Kapoor
27. LaareManinder Buttar
28. Me and My GirlfriendSidhu Moosewala
29. MexicoKaran Aujla
30. MuchhDiljit Dosanjh
31. OffshoreShubh
32. Patiala PegDiljit Dosanjh
33. Prada JassJass Manak
34. Putt Jatt DaDiljit Dosanjh
35. RoughDr Zeus
36. Sab Fade JangeParmish Verma
37. ShadaParmish Verma
38. SheikhKaran Aujla
39. Teeje WeekJordan Sandhu
40. These DaysSidhu Moosewala, Bohemia
41. TitliaanAfsana Khan, Harrdy Sandhu
42. Toxic APAP Dhillon
43. True StoriesAP Dhillon, Shinda Kahlon
44. Try MeKaran Aujla
45. Yaar AnmulleSharry Mann
46. Yaari ManinderManinder Buttar
3:59 AM 
    DIVINE 
    Aahun Aahun 
    Master Saleem, Neeraj Shridhar 
    Aaj Ki Raat 
    Sachin-Jigar, Madhubanti Bagchi, Divya Kumar 
    aaj raat ka scene (baadshah) 
    Unknown 
    Aala Re Aala 
    Simmba 
    Aankh Mare 
    Kumar Sanu, Kavita Krishnamurthy 
    Aankh Marey 
    Neha Kakkar, Mika Singh, Kumar Sanu, Tanishk Bagchi 
    Aankhon Mein Teri 
    KK 
    Aao Kabhi Haveli Pe 
    Badshah, Nikhita Gandhi 
    Aap Jaisa Koi Meri Zindagi 
    Nazia Hassan 
    Aap Jaisa Koi Remix 
    Zahrah S Khan, Altamash Faridi 
    Aarambh Hai Prachand 
    Piyush Mishra 
    Aashiq Banaya Aapne 
    Himesh Reshammiya, Shreya Ghoshal 
    Aashiq Surrender Hua 
    Amaal Mallik, Shreya Ghoshal 
    Aaya Re Toofan 
    A. R. Rahman 
    Aayi Nai 
    Pawan Singh, Simran Choudhary, Sachin-Jigar 
    Abhi Toh Party Shuru Hui Hai 
    Badshah, Aastha Gill 
    Achko Machko 
    Yo Yo Honey Singh 
    Afghan Jalebi 
    Asrar 
    Aila Re Aillaa 
    Daler Mehndi, Tanishk Bagchi 
    Ainvayi Ainvayi 
    Salim Merchant, Sunidhi Chauhan 
    Aithey Aa 
    Akasa Singh, Neeti Mohan, Kamaal Khan 
    Akhiyaan Gulaab 
    Mitraz 
    Albela Sajan 
    Shashi Suman, Kunal Pandit, Prithvi Gandharv 
    alka yagnik 
    Unknown 
    Ami Je Tomar Dance 
    Arijit Singh 
    Anarkali Disco Chali 
    Mamta Sharma, Sukhwinder Singh 
    Angad 
    Raanjha 
    Ankhiyon Se Goli Maare 
    Sonu Nigam, Jaspinder Narula 
    Apna Bana Le 
    Arijit Singh, Sachin-Jigar 
    Apna Time Aayega 
    Ranveer Singh, DIVINE 
    Arabic Kuthu 
    Anirudh Ravichander, Jonita Gandhi 
    Azadi 
    DIVINE, Dub Sharma 
    Baaki Baatein Peene Baad 
    Arjun Kanungo, Badshah 
    Baari Barsi 
    Harshdeep Kaur, Labh Janjua 
    Baazigar 
    DIVINE, Armani White 
    Baby Doll 
    Meet Bros Anjjan, Kanika Kapoor 
    Baby Ko Bass Pasand Hai 
    Badshah, Vishal Dadlani 
    Bachna Ae Haseeno Original 
    Kishore Kumar 
    Bachna Ae Haseeno Title 
    Kishore Kumar, Sumit Kumar, Vishal Dadlani 
    Bad Boy 
    Badshah, Neeti Mohan 
    Bade Miyan Chote Miyan 
    Udit Narayan, Sudesh Bhosle 
    Badri Ki Dulhania 
    Dev Negi, Neha Kakkar, Monali Thakur 
    Badtameez Dil 
    Benny Dayal, Shefali Alvares, Pritam 
    Bajrang Baan 
    Agam 
    Balam Pichkari 
    Vishal Dadlani, Shalmali Kholgade 
    Ban Ja Rani 
    Guru Randhawa 
    Bana Sharabi 
    Jubin Nautiyal 
    Bande Hain Hum Uske 
    Shivam Mahadevan, Anish Sharma 
    Bang Bang 
    Benny Dayal, Neeti Mohan 
    Bangles 
    Sanju Rathod, Isha Malviya 
    Banno Tera Swagger 
    Swati Sharma, Brijesh Shandllya 
    Bardaasht 
    Sunidhi Chauhan & KK 
    Bas Ek Kinng 
    Mika Singh, Neeraj Shridhar, Hard Kaur 
    Beedi Jalaile 
    Sunidhi Chauhan, Sukhwinder Singh 
    Befikra 
    Meet Bros, Aditi Singh Sharma 
    Besharam Rang 
    Shilpa Rao, Caralisa Monteiro, Vishal-Shekhar 
    Bezubaan Phir Se 
    Vishal Dadlani, Anushka Manchanda 
    Bhangra Ta Sajda 
    Neha Kakkar, Romy 
    Bholi Bhali Ladki 
    Udit Narayan, Alka Yagnik 
    Bhool Bhulaiyaa 2 Title 
    Neeraj Shridhar, Mellow D 
    Bhootni Ke 
    Daler Mehndi & Pritam 
    Bijli 
    Mika Singh, Neha Kakkar 
    Binte Dil 
    Arijit Singh 
    Biwi No. 1 Title Track 
    Abhijeet, Poornima 
    Blue Eyes 
    Yo Yo Honey Singh 
    Bole Chudiyan 
    Sonu Nigam, Alka Yagnik, Amit Kumar, Udit Narayan, Kavita Krishnamurthy 
    Bolo Har Har Har 
    Mithoon, Badshah 
    Bom Diggy Diggy 
    Zack Knight, Jasmin Walia 
    Brothers Anthem 
    Ajay-Atul, Vishal Dadlani 
    Brown Rang 
    Yo Yo Honey Singh 
    Burj Khalifa 
    Shashi - Dj Khushi, Nikhita Gandhi 
    Buzz 
    Aastha Gill, Badshah 
    Chaiyya Chaiyya 
    Sukhwinder Singh, Sapna Awasthi 
    Chak De India 
    Sukhwinder Singh 
    Chak Lein De 
    Kailash Kher 
    Chaka Chaka 
    Shreya Ghoshal 
    Chal Bombay 
    DIVINE 
    Chal Pyar Karegi 
    Sonu Nigam, Alka Yagnik 
    Chale Jaise Hawayein 
    KK, Vasundhara Das 
    Chaleya Dance Mix 
    Arijit Singh, Shilpa Rao, Anirudh 
    Chalka Chalka Re 
    Richa Sharma, Mahalaxmi Iyer 
    Cham Cham 
    Meet Bros, Monali Thakur 
    Chamma Chamma Baaje Re 
    Alka Yagnik, Shankar Mahadevan 
    Chammak Challo 
    Akon, Hamsika Iyer 
    Chand Chupa Badal Mein 
    Udit Narayan, Alka Yagnik 
    Chandi Ki Daal Par 
    Salman Khan, Alka Yagnik 
    Chandigarh Mein 
    Badshah, Harrdy Sandhu, Lisa Mishra 
    Channa Mereya 
    Arijit Singh, Pritam 
    Character Dheela 2.0 
    Neeraj Shridhar, Style Bhai 
    Chhote Chhote Bhaiyon Ke 
    Kavita Krishnamurthy, Udit Narayan, Kumar Sanu 
    Chikni Chameli 
    Shreya Ghoshal, Ajay-Atul 
    Chittiyaan Kalaiyaan 
    Meet Bros, Kanika Kapoor 
    Chogada 
    Darshan Raval, Asees Kaur, Lijo George, DJ Chetas 
    Choli Ke Peeche 
    Alka Yagnik, Ila Arun 
    Chor Bazari 
    Neeraj Shridhar, Sunidhi Chauhan 
    Chori Chori Jab Nazrein Mili 
    Kumar Sanu, Sanjivani 
    Chota Bachha Jaan Ke 
    Aditya Narayan 
    Chote Chote Peg 
    Yo Yo Honey Singh, Neha Kakkar, Navraj Hans 
    Chunari Chunari 
    Abhijeet, Anuradha Sriram 
    Chunari Sambhal Gori 
    Kumar Sanu, Kavita Krishnamurthy 
    Chura Ke Dil Mera 
    Kumar Sanu, Alka Yagnik 
    Chura Liya Hai Tumne Jo Dil Ko 
    Asha Bhosle, Mohammed Rafi 
    Coca Cola Tu 
    Tony Kakkar, Neha Kakkar 
    Coka 
    Sukhe Muzical Doctorz 
    Coka 2.0 
    Sukh-E Muzical Doctorz, Lisa Mishra 
    Crazy Kiya Re 
    Sunidhi Chauhan 
    Cutiepie 
    Pardeep Singh Sran, Nakash Aziz, Pritam 
    Daaru Desi 
    Benny Dayal, Shalmali Kholgade 
    Dance Ka Bhoot 
    Arijit Singh 
    Dance Meri Rani 
    Guru Randhawa, Zahrah S Khan 
    Dance Pe Chance 
    Sunidhi Chauhan, Labh Janjua 
    Dangal Title Track 
    Daler Mehndi 
    Dard-e-Disco 
    Sukhwinder Singh, Marianne DCruz 
    Deewana Main Chala 
    Kumar Sanu 
    Deewangi Deewangi 
    Shaan, Udit Narayan, Shreya Ghoshal 
    Deewani Mastani 
    Shreya Ghoshal 
    Dekha Hai Pehli Baar 
    S. P. Balasubrahmanyam, Alka Yagnik 
    Dekho Nashe Mein 
    Sunidhi Chauhan, Shaan 
    Desi Girl 
    Shankar Mahadevan, Sunidhi Chauhan, Vishal Dadlani 
    Desi Kalakaar 
    Yo Yo Honey Singh 
    Deva Deva 
    Arijit Singh, Pritam 
    Deva Shree Ganesha 
    Ajay-Atul 
    Dhaakad 
    Raftaar 
    Dhan Te Nan 
    Sukhwinder Singh, Vishal Dadlani 
    Dheeme Dheeme 
    Tony Kakkar, Neha Sharma 
    Dheera Dheera 
    Ananya Bhat 
    Dheere Dheere Se Meri Zindagi 
    Yo Yo Honey Singh 
    Dhindhora Baje Re 
    Darshan Raval, Bhoomi Trivedi 
    Dhol Bajne Laga 
    Udit Narayan, Kavita Krishnamurthy 
    Dholi Taro Dhol Baaje 
    Kavita Krishnamurthy, Vinod Rathod, Karsan Sagathia 
    Dhoom Again 
    Vishal Dadlani, Dominique Cerejo 
    Dhoom Machale 
    Sunidhi Chauhan 
    Dhoom Taana 
    Abhijeet Bhattacharya, Shreya Ghoshal 
    Didi Tera Devar Deewana 
    Lata Mangeshkar, S. P. Balasubrahmanyam 
    Dil Chahta Hai 
    Shankar Mahadevan, Shankar-Ehsaan-Loy 
    Dil Chori 
    Yo Yo Honey Singh, Simar Kaur, Ishers 
    Dil Diyan Gallan Remix 
    Atif Aslam, DJ Chetas 
    Dil Galti Kar Baitha Hai 
    Jubin Nautiyal 
    Dil Laga Na 
    Sukhwinder Singh, Soham Chakraborty, Jolly Mukherjee 
    Dil Na Diya 
    Kunal Ganjawala 
    Dil Tote Tote Ho Gaya 
    Shweta Shetty, Hans Raj Hans 
    Dilbar 
    Neha Kakkar, Dhvani Bhanushali, Ikka 
    Dilbar Dilbar 
    Alka Yagnik 
    Dilbara 
    Abhijeet, Sowmya Raoh 
    Dilliwaali Girlfriend 
    Arijit Singh, Sunidhi Chauhan, Pritam 
    Dilruba Dilnasheen I Dilfek EP I Rito Riba 
    RITO RIBA 
    Ding Dang 
    Amit Mishra, Antara Mitra 
    DJ Waley Babu 
    Badshah, Aastha Gill 
    Do You Know Housefull 
    Shaan, Shreya Ghoshal 
    Dola Re Dola 
    Kavita Krishnamurthy, Shreya Ghoshal 
    Dole Dole Dil Dole 
    Kavita Krishnamurthy 
    Drama Queen 
    Shreya Ghoshal, Vishal Dadlani 
    Dulhe Ka Sehra 
    Nusrat Fateh Ali Khan 
    Dum Dum 
    Benny Dayal, Himani Kapoor 
    Dum Maro Dum 
    Asha Bhosle 
    Duniya Haseenon Ka Mela 
    Udit Narayan, Suneeta Rao 
    Duniya Mein Aaye Ho To Love Karlo 
    Kumar Sanu, Kavita Krishnamurthy 
    Dus Bahane 2.0 
    Vishal & Shekhar, KK, Shaan 
    Dus Bahane Original 
    Shaan, KK 
    Ek Garam Chai Ki Pyali Ho 
    Anu Malik 
    Ek Haseena Thi 
    Kishore Kumar, Asha Bhosle 
    Ek Ho Gaye Hum Aur Tum 
    Remo Fernandes 
    Ek Pal Ka Jeena 
    Lucky Ali 
    Engine Ki Seeti 
    Sunidhi Chauhan, Resmi Sateesh 
    Enni Soni 
    Guru Randhawa, Tulsi Kumar 
    Fan Anthem 
    Nakash Aziz 
    Fashion Ka Jalwa 
    Sukhwinder Singh, Satya Hinduja 
    Firse Machayenge 
    Emiway Bantai 
    First Class 
    Arijit Singh, Neeti Mohan 
    Gal Mitthi Mitthi Bol 
    Tochi Raina, Amit Trivedi 
    Gallan Goodiyaan 
    Yashita Sharma, Manish Kumar Tipu, Farhan Akhtar, Shankar Mahadevan, Sukhwinder Singh 
    Gallan Goriyan 
    Dhvani Bhanushali, Taz 
    Gallan Kardi 
    Jazzy B, Jyotica Tangri, Mumzy Stranger 
    Garmi 
    Badshah, Neha Kakkar 
    Get Ready To Fight 
    Vishal Dadlani 
    Ghamand Kar 
    Sachet Tandon, Parampara Tandon 
    Ghani Bawri 
    Jyoti Nooran, Krsna Solo 
    Ghoomar 
    Shreya Ghoshal, Swaroop Khan 
    Ghungroo 
    Arijit Singh, Shilpa Rao, Vishal-Shekhar 
    Goa Beech 
    Tony Kakkar, Neha Kakkar 
    Gore Gore Gaal Mere 
    Alisha Chinai 
    Gori Gori 
    KK, Shreya Ghoshal, Sunidhi Chauhan, Anu Malik 
    Goriya Re Goriya 
    Udit Narayan, Poornima 
    Gulaabo 
    Vishal Dadlani, Anusha Mani, Amit Trivedi 
    Haan Main Galat 
    Arijit Singh, Shashwat Singh 
    Hai Hai Mirchi 
    Sukhwinder Singh, Alka Yagnik 
    Har Har Gange 
    Arijit Singh 
    Har Har Shambhu Shiv Mahadeva 
    Abhilipsa Panda, Jeetu Sharma 
    Hata Sawan Ki Ghata 
    Babul Supriyo, Jaspinder Narula 
    Haule Haule 
    Sukhwinder Singh 
    Hawa Hawa 
    Mika Singh, Prakriti Kakar 
    Hello Brother Title Track 
    Kamaal Khan 
    Heyy Babyy Title 
    Neeraj Shridhar, Raman Mahadevan 
    High Heels Te Nachche 
    Meet Bros, Yo Yo Honey Singh, Jaz Dhami 
    Holi Ke Din 
    Kishore Kumar, Lata Mangeshkar 
    Hookah Bar 
    Himesh Reshammiya, Vineet Singh, Aman Trikha 
    Humma Humma (Classic) 
    Remo Fernandes, A.R. Rahman 
    Husn Hai Suhana 
    Chandana Dixit, Abhijeet Bhattacharya 
    I Am A Disco Dancer 
    Vijay Benedict 
    Idhar Chala Main Udhar Chala 
    Udit Narayan, Alka Yagnik 
    Ilahi 
    Arijit Singh, Pritam 
    Illegal Weapon 2.0 
    Jasmine Sandlas, Garry Sandhu 
    Ishare Tere 
    Guru Randhawa, Dhvani Bhanushali 
    Ishq Hua Kaise Hua 
    Udit Narayan, Vibha Sharma 
    Ishq Jaisa Kuch 
    Vishal & Sheykhar, Mellow D, Shilpa Rao 
    Ishq Sona Hai 
    Shankar Mahadevan, Hema Sardesai 
    Iski Uski 
    Shahid Mallya, Akriti Kakar 
    It's The Time To Disco 
    Shaan, Vasundhara Das, KK, Loy Mendonsa 
    Jaanam Samjha Karo 
    Anu Malik, Hema Sardesai 
    Jaane Kyun 
    Vishal Dadlani 
    Jaanu Meri Jaan 
    Mohammed Rafi, Kishore Kumar, Asha Bhosle 
    Jai Ho 
    A. R. Rahman, Sukhwinder Singh 
    Jai Jai Shivshankar 
    Vishal Dadlani, Benny Dayal 
    Jai Shri Ram Adipurush 
    Ajay-Atul 
    Jamal Kudu 
    Animal 
    Janaabe Ali 
    Shaan & Kunal Ganjawala 
    Jat Jaguar 
    Vishal Dadlani, Navraj Hans 
    Jatt Mehkma 
    Yo Yo Honey Singh 
    Jawani Jan-E-Man 
    Asha Bhosle 
    Jedha Nasha 
    Amar Jalal, IP Singh, Yohani 
    Jee Karda 
    Labh Janjua, Suzie Q 
    Jee Karda Badlapur 
    Divya Kumar 
    Jhalak Dikhla Jaa 
    Himesh Reshammiya 
    Jhanjharia 
    Abhijeet Bhattacharya, Anu Malik 
    Jhoome Jo Pathaan 
    Arijit Singh, Sukriti Kakar 
    Jimmy Jimmy Aaja 
    Parvati Khan 
    Jiya Tu Bihar Ke Lala 
    Gangs Of Wasseypur 
    Jogi Mahi 
    Sukhwinder Singh, Shekhar Ravjiani 
    Joote Do Paise Lo 
    Lata Mangeshkar, S.P. Balasubrahmanyam 
    Jugni 
    Mika Singh 
    Jugnu 
    Badshah, Nikhita Gandhi 
    Jumma Chumma De De 
    Sudesh Bhosle, Kavita Krishnamurthy 
    Jungle Hai Aadhi Raat Hai 
    Kumar Sanu, Hema Sardesai 
    Jungle Jungle Baat Chali Hai 
    Amol Sahdev 
    Kaam 25 
    DIVINE 
    Kabira (Encore) 
    Harshdeep Kaur, Arijit Singh, Pritam 
    Kaho Na Pyar Hai Remix 
    Udit Narayan, DJ Aqeel 
    Kaho Naa Pyaar Hai Title 
    Udit Narayan, Alka Yagnik 
    Kajra Re 
    Alisha Chinai, Shankar Mahadevan, Javed Ali 
    Kala Chashma 
    Amar Arshi, Badshah, Neha Kakkar 
    Kalaastar 
    Yo Yo Honey Singh 
    Kalyani 
    ARJN, KDS, Shreya Ghoshal 
    Kamariya Mitron 
    Darshan Raval 
    Kamariya Stree 
    Aastha Gill, Divya Kumar 
    Kamli 
    Sunidhi Chauhan 
    Kandhon Se Milte Hain Kandhe 
    Lakshya 
    Kanta Laga 
    Tony Kakkar, Yo Yo Honey Singh, Neha Kakkar 
    Kar Gayi Chull 
    Badshah, Fazilpuria, Sukriti Kakar, Neha Kakkar 
    Kar Har Maidaan Fateh 
    Sukhwinder Singh, Shreya Ghoshal 
    Kaun Nachdi 
    Guru Randhawa, Neeti Mohan 
    Kay Sera Sera 
    Shankar Mahadevan, Kavita Krishnamurthy 
    Khaike Paan Banaraswala 
    Kishore Kumar 
    Khalibali 
    Shivam Pathak, Shail Hada 
    Khoobsurat 
    Vishal Mishra, Sachin-Jigar 
    Khoon Chala 
    A. R. Rahman 
    Khuda Jaane 
    KK, Shilpa Rao 
    Kisi Disco Mein Jaaye 
    Alka Yagnik, Udit Narayan 
    Kiya Kiya 
    Anand Raj Anand, Shweta Pandit 
    Kohinoor 
    DIVINE 
    Koi Jaye To Le Aaye 
    Alka Yagnik, Shankar Mahadevan 
    Koi Kahe Kehta Rahe 
    Shankar Mahadevan, Shaan, KK 
    Koi Ladki Hai (Chak Dhum Dhum) 
    Udit Narayan, Lata Mangeshkar 
    Koi Mil Gaya 
    Udit Narayan, Alka Yagnik 
    Kudi Kanwari 
    Jaspinder Narula 
    Kudi Nu Nachne De 
    Sachin-Jigar, Vishal Dadlani, Sachin Sanghvi 
    Kudmayi 
    Shahid Mallya, Pritam 
    Kurta Pajama 
    Tony Kakkar 
    Kusu Kusu 
    Zahrah S Khan, Dev Negi 
    Kyaa Baat Haii 2.0 
    Harrdy Sandhu, Nikhita Gandhi 
    Laal Peeli Akhiyaan 
    Romy, Tanishk Bagchi 
    Ladki Badi Anjani Hai 
    Kumar Sanu, Alka Yagnik 
    Lahore 
    Guru Randhawa 
    Laila O Laila 
    Amit Kumar, Kanchan 
    Lakshya Title Track 
    Shankar Mahadevan 
    Lamberghini 
    The Doorbeen, Ragini Tandan 
    Lat Lag Gayee 
    Benny Dayal, Shalmali Kholgade 
    Le Gayi 
    Uttam Singh, Asha Bhosle 
    Lehra Do 
    Arijit Singh 
    Lekar Hum Deewana Dil 
    Asha Bhosle, Kishore Kumar 
    Leke Prabhu Ka Naam 
    Arijit Singh, Nikhita Gandhi 
    Let's Nacho 
    Badshah, Benny Dayal, Nucleya 
    Lo Chali Main 
    Lata Mangeshkar 
    Loca 
    Yo Yo Honey Singh 
    Locha-E-Ulfat 
    Benny Dayal 
    Love Dose 
    Yo Yo Honey Singh 
    Love Hua 
    Kumar Sanu, Alka Yagnik 
    Love Mera Hit Hit 
    Neeraj Shridhar, Tulsi Kumar 
    Lovely 
    Kanika Kapoor, Ravindra Upadhyay, Fateh 
    LTMP Reloaded Non Stop Mix 
    Sohail Sen, Shankar Mahadevan, Sunidhi Chauhan 
    Lucky Boy 
    Sunidhi Chauhan, Raja Hasan, Hard Kaur 
    Ludo 
    Tony Kakkar 
    Lungi Dance 
    Yo Yo Honey Singh 
    Lutt Putt Gaya 
    Arijit Singh 
    Maa Da Laadla 
    Master Saleem 
    Maahi Ve 
    Sadhana Sargam, Sujata Bhattacharya, Udit Narayan, Shankar-Ehsaan-Loy 
    Machayenge 
    Emiway Bantai 
    Machayenge 2 
    Emiway Bantai 
    Machayenge 3 
    Emiway Bantai 
    Machayenge 4 
    Emiway Bantai 
    Made In India 
    Alisha Chinai 
    Mahakaal 
    Raanjha 
    Main Hoon Na Title 
    Sonu Nigam, Shreya Ghoshal 
    Main Khiladi Tu Anari 
    Abhijeet, Udit Narayan 
    Main Laila Laila Bolunga 
    Abhijeet, Poornima 
    Main To Raste Se Jaa Raha Tha 
    Kumar Sanu, Alka Yagnik 
    Maiyya Yashoda 
    Anuradha Paudwal, Alka Yagnik, Kavita Krishnamurthy 
    Makhna Yo Yo 
    Yo Yo Honey Singh, Neha Kakkar, Singhsta 
    Malang 
    Siddharth Mahadevan, Shilpa Rao 
    Malhari 
    Vishal Dadlani 
    Manali Trance 
    Yo Yo Honey Singh, Neha Kakkar 
    Manchala 
    Shafqat Amanat Ali, Nupur Pant 
    Mantoiyat 
    Raftaar 
    Marjaani 
    Sukhwinder Singh, Sunidhi Chauhan 
    Mashallah 
    Wajid, Shreya Ghoshal 
    Mast Kalandar 
    Shankar Mahadevan, Sunidhi Chauhan, Sajid Khan 
    Mast Magan 
    Arijit Singh, Chinmayi Sripaada 
    Matargashti 
    Mohit Chauhan, A.R. Rahman 
    Mauja Hi Mauja 
    Mika Singh, Pritam 
    Mehbooba Mehbooba 
    R.D. Burman 
    Mehndi Laga Ke Rakhna 
    Lata Mangeshkar, Udit Narayan 
    Mera Dil Tera Deewana 
    Alka Yagnik 
    Mercy 
    Badshah 
    Mere Gully Mein 
    DIVINE, Naezy 
    Mere Khwabon Mein 
    Lata Mangeshkar 
    Mere Yaar Ki Shadi Hai (Remix) 
    Dj Karan Kahar & Dj Hariom Hkg 
    Mhare Hiwda Mein Naache Mor 
    Hariharan, Kumar Sanu, Alka Yagnik, Udit Narayan, Anuradha Paudwal 
    Mile Ho Tum Reprise 
    Neha Kakkar, Tony Kakkar 
    Milegi Milegi 
    Mika Singh 
    Millionaire 
    Yo Yo Honey Singh 
    Mirchi 
    DIVINE, MC Altaf, Phenom, Stylo G 
    Mitwa KANK 
    Shafqat Amanat Ali, Shankar Mahadevan, Caralisa Monteiro 
    Monster Song KGF 
    Adithi Sagar 
    Most Wanted Munda 
    Meet Bros, Earl Edgar 
    Mr. Lova Lova 
    Udit Narayan, Abhijeet, Sudesh Bhosle, Kavita Krishnamurthy 
    Mubarakan Title 
    Juggy D, Yash Narvekar, Badshah 
    Munda Sona Hoon Main 
    Diljit Dosanjh, Nikhita Gandhi 
    Muqabala Muqabala 
    Mano, Swarnalatha 
    Muqabla 
    Yash Narvekar, Parampara Thakur 
    My Dil Goes Mmm 
    Shaan, Gayatri Ganjawala 
    My Name Is Anthony Gonsalves 
    Kishore Kumar, Amitabh Bachchan 
    Na Na Na Re 
    Daler Mehndi, Sudesh Bhosle 
    Naach Meri Rani 
    Guru Randhawa, Nikhita Gandhi 
    Naagin 
    Aastha Gill, Akasa 
    Naatu Naatu 
    Rahul Sipligunj, Kaala Bhairava 
    Nach Baliye 
    Shankar Mahadevan, Sowmya Raoh 
    Nach Punjaban 
    Abrar Ul Haq 
    Nadiyon Paar 
    Shamur, Rashmeet Kaur, Sachin-Jigar 
    Nagada Nagada 
    Sonu Nigam, Javed Ali 
    Nainowale Ne 
    Neeti Mohan 
    Namak Ishq Ka 
    Rekha Bhardwaj, Rakesh Pandit 
    Namo Namo 
    Amit Trivedi 
    Nashe Si Chadh Gayi 
    Arijit Singh 
    Navrai Majhe 
    Sunidhi Chauhan, Swanand Kirkire, Neelambari Kirkire, Amit Trivedi 
    Nayak Nahi Khalnayak Hoon Main 
    Vinod Rathod 
    Neend Churayee Meri 
    Kumar Sanu, Udit Narayan, Alka Yagnik, Kavita Krishnamurthy 
    Nikle Currant 
    Jassi Gill, Neha Kakkar 
    Nimbooda Nimbooda 
    Karsan Sagathia, Kavita Krishnamurthy 
    Not Ramaiya Vastavaiya 
    Anirudh Ravichander 
    O O Jaane Jaana 
    Kamaal Khan 
    O Yaara Dhol Bajake (Reprise) 
    Pritam, Soham Chakraborty & Sohail Kaul 
    Offo 
    Aditi Singh Sharma, Benny Dayal 
    Ole Ole 
    Abhijeet Bhattacharya, Dilip Sen-Sameer Sen 
    Ole Ole 2.0 
    Amit Mishra 
    Om Shanti Om Karz 
    Kishore Kumar 
    Oo Antava Mawa 
    Indravathi Chauhan 
    Oonchi Hai Building 
    Poornima, Anu Malik 
    Paani Paani 
    Badshah, Aastha Gill 
    Pag Ghunghroo Baandh 
    Kishore Kumar 
    Paisa Paisa 
    RDB, Manak-E, Selina 
    Paisa Yeh Paisa 
    Kishore Kumar 
    Panghat 
    Asees Kaur, Divya Kumar, Sachin-Jigar 
    Papa Jag Jaayega 
    Neeraj Shridhar, Alyssa Mendonsa 
    Papa Toh Band Bajaye 
    Neeraj Shridhar 
    Param Sundari 
    Shreya Ghoshal 
    Parshuram 
    Raanjha 
    Party All Night 
    Yo Yo Honey Singh 
    Party Mashup - DJ Praveen - Nonie 
    Shruti Rane, Badshah, Aastha Gill, Jassie Gill, Simar Kaur & Dev Negi 
    Party On My Mind 
    KK, Shefali Alvares, Yo Yo Honey Singh 
    Patola 
    Guru Randhawa, Bohemia 
    Payal 
    Yo Yo Honey Singh, Paradox 
    Pehla Pehla Pyar 
    S.P. Balasubrahmanyam 
    Pehle Bhi Main Remix 
    Vishal Mishra, DJ Shadow 
    Pehli Nazar Mein 
    Atif Aslam 
    Pehli Pehli Baar Jab Pyaar Kisi Se Hota Hai 
    Kumar Sanu 
    Phatte Tak Nachna 
    Sunidhi Chauhan 
    Photo 
    Karan Sehmbi 
    Pichle Saat Dinon Mein 
    Farhan Akhtar 
    Piya Piya O Piya 
    Pinky, Preeti 
    Poster Lagwa Do 
    Mika Singh, Sunanda Sharma 
    Prada 
    The Doorbeen, Shreya Sharma 
    Pretty Woman 
    Shankar Mahadevan, Ravi Khote 
    Proper Patola 
    Diljit Dosanjh, Badshah, Aastha Gill 
    Psycho Saiyaan 
    Sachet Tandon, Dhvani Bhanushali 
    Punjabi Wedding Song 
    Sunidhi Chauhan, Benny Dayal 
    Pyaar Hota Kayi Baar Hai 
    Arijit Singh 
    Pyar Dilon Ka Mela Hai 
    Alka Yagnik, Sonu Nigam 
    Raat Baaqi Baat Baaqi 
    Bappi Lahiri, Asha Bhosle 
    Raat Ke Dhai Baje 
    Suresh Wadkar, Rekha Bhardwaj, Sunidhi Chauhan 
    Raataan Lambiyan 
    Tanishk Bagchi, Jubin Nautiyal, Asees Kaur 
    Race Is On My Mind 
    Sunidhi Chauhan, Neeraj Shridhar 
    Race Saanson Ki 
    Sunidhi Chauhan, Neeraj Shridhar 
    Radha Nachegi 
    Ritu Pathak, Danish Sabri 
    Ram aa Gaye 
    Payal Dev & Pawan Singh 
    Rang De 
    Asha Bhosle 
    Rang De Basanti 
    Daler Mehndi, Chitra 
    Ravan Ravan 
    Raanjha 
    Right Here Right Now 
    Abhishek Bachchan, Sunidhi Chauhan 
    Right Now Now 
    Wajid, Sunidhi Chauhan 
    Rock N Roll Soniye 
    Shankar Mahadevan, Shaan, Mahalakshmi Iyer 
    Rock On Title 
    Farhan Akhtar 
    Roke Na Ruke Naina 
    Arijit Singh 
    Ruk Ja O Dil Deewane 
    Udit Narayan 
    Saajanji Ghar Aaye 
    Kumar Sanu, Alka Yagnik, Kavita Krishnamurthy 
    Saami Saami 
    Sunidhi Chauhan 
    Saat Samundar Paar 
    Sadhana Sargam 
    Sadda Haq 
    Mohit Chauhan 
    Sadi Gali 
    Lehmber Hussainpuri 
    Salaam Namaste Title 
    Kunal Ganjawala, Vasundhara Das 
    Salam Rocky Bhai 
    Vijay Prakash, Santhosh Venky 
    Sanak 
    Badshah 
    Sankatmochan Hanuman 
    Raanjha 
    Sanson Ki Mala 
    Pritam Ka Kuch Dosh Nahi 
    Sasural Genda Phool 
    Rekha Bhardwaj, Shraddha Pandit, Sujata Majumdar 
    Satranga Dance Mix 
    Arijit Singh, DJ Chetas 
    Saturday Saturday 
    Badshah, Indeep Bakshi, Akriti Kakar 
    Savariya 
    Kamal Khan, Alka Yagnik 
    Say Shava Shava 
    Sudesh Bhosle, Alka Yagnik, Sunidhi Chauhan, Aadesh Shrivastava 
    Senorita 
    Farhan Akhtar, Hrithik Roshan, Abhay Deol, Maria del Mar Fernandez 
    Shaam Shaandaar 
    Amit Trivedi 
    Shaitan Ka Saala 
    Vishal Dadlani 
    Shake It Like Shammi 
    Benny Dayal 
    Shankara Re Shankara 
    Mehul Vyas 
    Sher Aaya Sher 
    DIVINE 
    Sher Khul Gaye 
    Vishal & Shekhar, Benny Dayal 
    Shikdum 
    Shaan, Shreya Ghoshal 
    Shiv Tandav Stotram 
    Agam 
    Shona Shona 
    Tony Kakkar, Neha Kakkar 
    Show Me The Thumka 
    Sunidhi Chauhan, Shashwat Singh 
    Shringaar 
    Akasa, Aastha Gill, Raftaar 
    Shubhaarambh 
    Shruti Pathak, Divya Kumar, Amit Trivedi 
    Shut Up & Bounce 
    Sunidhi Chauhan, Vishal Dadlani 
    Sinbad The Sailor 
    Farhan Akhtar, Raman Mahadevan 
    Singh Is Kinng Title 
    Snoop Dogg, RDB, Akshay Kumar 
    Singham 
    Ajay-Atul 
    Slow Motion 
    Nakash Aziz, Shreya Ghoshal 
    Slowly Slowly 
    Guru Randhawa, Pitbull 
    Soch Na Sake 
    Arijit Singh, Tulsi Kumar, Amaal Mallik 
    Socha Hai 
    Farhan Akhtar 
    Sona Kitna Sona Hai 
    Udit Narayan, Poornima 
    Sona Sona 
    Sonu Nigam, Sudesh Bhonsle 
    Sooraj Dooba Hain 
    Arijit Singh, Aditi Singh Sharma, Amaal Mallik 
    Sooraj Ki Baahon Mein 
    Clinton Cerejo, Dominique Cerejo 
    Soorma Anthem 
    Shankar Mahadevan 
    Soulmate 
    Badshah, Arijit Singh 
    Srivalli 
    Javed Ali 
    Subah Hone Na De 
    Mika Singh, Shefali Alvares, Pritam 
    Subah Subah 
    Arijit Singh, Prakriti Kakar 
    Sultan Title Track 
    Sukhwinder Singh, Shadab Faridi 
    Sulthan 
    KGF Chapter 2 
    Sunny Sunny 
    Yo Yo Honey Singh, Neha Kakkar 
    Swag Mera Desi 
    Raftaar 
    Swag Se Swagat 
    Vishal Dadlani, Neha Bhasin 
    Sweety Tera Drama 
    Dev Negi, Pawni Pandey, Shraddha Pandit, Tanishk Bagchi 
    Taal Se Taal 
    Sukhwinder Singh 
    Taang Uthake 
    Sohail Sen, Mika Singh, Neeti Mohan 
    Tabaahi 
    Toxic (Hindi) 
    Tamma Tamma Again 
    Badshah, Bappi Lahiri, Anuradha Paudwal 
    Tan Tana Tan Tan 
    Abhijeet Bhattacharya, Poornima 
    Tareefan 
    Badshah 
    Tenu Leke Main Javanga 
    Sonu Nigam, Mahalakshmi Iyer 
    Tera Rang Balle Balle 
    Jaspinder Narula, Sonu Nigam 
    Teri Aakhya Ka Yo Kajal 
    DC Madana, Sapna Choudhary 
    Teri Baaton Mein Aisa Uljha Jiya Title 
    Raghav, Tanishk Bagchi, Asees Kaur 
    Teri Chunnariya 
    Kumar Sanu, Alka Yagnik 
    The Breakup Song 
    Arijit Singh, Badshah, Jonita Gandhi 
    The Disco Song 
    Sunidhi Chauhan, Benny Dayal, Nazia Hassan, Vishal-Shekhar 
    The Goggle Song 
    Sonu Nigam, Armaan Malik, Amaal Mallik 
    The Humma Song 
    Jubin Nautiyal, Shashaa Tirupati, Badshah, A.R. Rahman 
    The Punjaabban Song 
    Gippy Grewal, Zahrah S Khan 
    Thodasa Pagla 
    Asha Bhosle 
    Thodisi Jo Pee Lee Hai 
    Kishore Kumar 
    Tip Tip Barsa Paani 
    Alka Yagnik, Udit Narayan 
    Toofan 
    KGF Chapter 2 
    Touch Me Touch Me 
    Alisha Chinai 
    Tu Cheez Badi Hai Mast Mast 
    Udit Narayan, Kavita Krishnamurthy 
    Tum Hi Ho Bandhu 
    Neeraj Shridhar, Kavita Seth 
    Tum Par Hum Hai Atke Yaara 
    Kumar Sanu, Kavita Krishnamurthy 
    Tumse Milke Dil Ka 
    Sonu Nigam, Aftab Sabri, Hashim Sabri 
    Twist 
    Neeraj Shridhar 
    Twist Kamariya 
    Harshdeep Kaur, Yasser Desai 
    Ude Dil Befikre 
    Benny Dayal 
    Uff Teri Adaa 
    Shankar Mahadevan 
    Uncha Lamba Kad 
    Anand Raj Anand, Kalpana Patowary 
    UP Bihar Lootne 
    Sapna Awasthi, Sukhwinder Singh 
    Urvashi Urvashi 
    A.R. Rahman, Suresh Peters, Shahul Hameed 
    Vaathi Coming 
    Anirudh Ravichander 
    Voodoo 
    Badshah, J Balvin, Tainy 
    Wah Wah Ramji 
    Lata Mangeshkar, S.P. Balasubrahmanyam 
    Wahi Abhimanyu Hu Mai 
    Ravan 
    Wahi Karn Hu Mai 
    Ravan 
    Welcome Title 
    Shaan, Soumya Raoh, Wajid 
    What Is Mobile Number 
    Sonu Nigam, Alka Yagnik 
    What Jhumka? 
    Arijit Singh, Jonita Gandhi, Pritam 
    Where's The Party Tonight 
    Shaan, Vasundhara Das, Loy Mendonsa, Shankar Mahadevan 
    Woh Ladki Hai Kahan 
    Shaan, Kavita Krishnamurthy 
    Yaad Aa Raha Hai 
    Bappi Lahiri 
    Yaar Naa Miley 
    Yo Yo Honey Singh, Jasmine Sandlas 
    Yamma Yamma 
    Mohammed Rafi, R.D. Burman 
    Yeh Dosti Hum Nahi Todenge 
    Kishore Kumar, Manna Dey 
    Yeh Ishq Hai 
    Shreya Ghoshal 
    Yeh Ladka Hai Allah 
    Udit Narayan, Alka Yagnik 
    Yeh Ladka Hai Deewana 
    Udit Narayan, Alka Yagnik 
    Yeh Mera Dil Yaar Ka Deewana 
    Asha Bhosle 
    You Are My Soniya 
    Sonu Nigam, Alka Yagnik 
    Yun Hi Chala Chal 
    Swades 
    Zara Sa Jhoom Loon Main 
    Asha Bhosle, Abhijeet 
    Zara Zara Touch Me 
    Monali Thakur 
    Zaroori Tha 
    Rahat Fateh Ali Khan 
    Zehnaseeb 
    Chinmayi Sripaada, Shekhar Ravjiani 
    Ziddi Dil 
    Vishal Dadlani 
    Zinda 
    Siddharth Mahadevan 
    Zinda Banda 
    Anirudh Ravichander
"""

known_artists = [
    "Sharry Mann, Mista Baaz", "Yo Yo Honey Singh, Gippy Grewal", "Jassi Gill", "Diljit Dosanjh",
    "Sharry Mann", "Kaka Bhainia Wala", "Master Saleem, Panjabi MC", "Gurnam Bhullar",
    "Yo Yo Honey Singh, Deep Money", "Harbhajan Mann", "Navraj Hans, Harshdeep Kaur, Rochak Kohli",
    "Jordan Sandhu", "Maninder Buttar", "Sukhbir", "Karan Aujla", "Mankirt Aulakh", "Dr Zeus",
    "Bally Sagoo, Rama", "Jass Manak", "Labh Janjua, Sonu Kakkar, Neha Kakkar",
    "Tanishk Bagchi, Yasser Desai, Asees Kaur", "Panjabi MC", "Guru Randhawa, Neha Kakkar, Tanishk Bagchi",
    "Yuvraj Hans", "Jasleen Royal, Harshdeep Kaur, Siddharth Mahadevan", "Maninder Buttar, Asees Kaur",
    "RDB", "Diljit Dosanjh, Sukhbir, Dhvani Bhanushali", "Mika Singh", "Guru Randhawa, Arjun",
    "Amrit Maan", "Daler Mehndi", "Sukh-E Muzical Doctorz, Raftaar", "AP Dhillon",
    "Karan Aujla, Y Shorty", "Shubh", "Yo Yo Honey Singh", "Kamal Kahlon, Param Singh",
    "Jasbir Jassi", "Dr Zeus, Rouge", "AP Dhillon, Gurinder Gill", "Sidhu Moosewala",
    "Meet Bros, Sukhbir, Neha Kakkar, Yo Yo Honey Singh", "Malkit Singh, Bally Sagoo",
    "Harrdy Sandhu", "Sukh-E Muzical Doctorz, Bohemia", "Dr Zeus, Kanika Kapoor",
    "Parmish Verma", "Sidhu Moosewala, Bohemia", "Afsana Khan, Harrdy Sandhu",
    "AP Dhillon, Shinda Kahlon"
]

# Sort known artists by length descending so longer matches take precedence
known_artists.sort(key=len, reverse=True)

songs = []
lines = [l.strip() for l in raw_text.strip().split('\n') if l.strip()]

i = 0
while i < len(lines):
    line = lines[i]
    
    # Special case for "3:59 AM" and "DIVINE"
    if line == "3:59 AM":
        if i + 1 < len(lines) and lines[i+1] == "DIVINE":
            songs.append({"title": "3:59 AM", "artist": "DIVINE"})
            i += 2
            continue
            
    m = re.match(r'^\d+\.\s*(.+)$', line)
    if m:
        content = m.group(1).strip()
        matched_artist = None
        matched_title = None
        for a in known_artists:
            if content.endswith(a):
                matched_title = content[:-len(a)].strip()
                matched_artist = a
                break
        if matched_artist:
            songs.append({"title": matched_title, "artist": matched_artist})
        else:
            songs.append({"title": content, "artist": "Unknown"})
        i += 1
        continue
        
    if i + 1 < len(lines):
        title = line
        artist = lines[i+1]
        songs.append({"title": title, "artist": artist})
        i += 2
    else:
        i += 1

print(f"Total structured songs parsed: {len(songs)}")

# Check duplicates by title
titles = {}
for idx, s in enumerate(songs):
    t_lower = s['title'].lower()
    if t_lower in titles:
        titles[t_lower].append(idx)
    else:
        titles[t_lower] = [idx]

dupes = {k: v for k, v in titles.items() if len(v) > 1}
print(f"Duplicate titles found: {len(dupes)}")
for d, idxs in dupes.items():
    print(f"  -> '{d}': indices {idxs} ({[songs[i]['artist'] for i in idxs]})")

out_path = r'c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\curated_616_songs.json'
with open(out_path, 'w', encoding='utf-8') as f:
    json.dump(songs, f, indent=2, ensure_ascii=False)
print(f"Saved to {out_path}")
