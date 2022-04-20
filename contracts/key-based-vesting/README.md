# Key-based Vesting

This contract is to provide vesting account features for the both cw20 and native tokens.


# Operations

## Execute register_vesting_account

베스팅 스케쥴 등록하기.

```json
{
  "register_vesting_account": {
    "address": "terra1...ylya",         // 수신자 주소
    "master_address": "terra1...0r7f",  // 관리자 주소 (베스팅 스케쥴 취소 가능)
    "vesting_key": "test1",             // 중복되지 않는 베스팅 키
    "vesting_schedule": {               // 베스팅 스케쥴 종류
      ...
    }
  }
}
```

스케쥴을 등록할 때 베스팅할 토큰 물량 전부를 같이 전송해줘야 함.
전송되는 토큰의 denom으로 베스팅이 수행 됨.

cw20 토큰으로 등록하기 위해서는 토큰을 보유한 계정이
베스팅 컨트랙트로 물량을 보내면서 베스팅 등록 메시지를 담아서 전송해야 함.
```javascript
  const vesting_msg = {  // 베스팅 등록 메시지
    register_vesting_account: {
      address: 'terra1...ylya',
      master_address: 'terra1...0r7f',
      vesting_key: 'test4',
      vesting_schedule: {
        linear_vesting: {
          start_time: '' + ((new Date('2022-04-15T05:00:00.000Z')).getTime() / 1000),
          end_time: '' + ((new Date('2022-04-15T05:10:00.000Z')).getTime() / 1000),
          vesting_amount: '100000',
        }
      },
    },
  };

  const execute = new MsgExecuteContract(
    'terra1...c08z',  // 물량을 보내는 계정
    'terra1...f49y',  // cw20 토큰 컨트랙트
    {
      send: {
        amount: '100000',           // 총 베스팅 물량
        contract: 'terra1...qgaw',  // 베스팅 컨트랙트
        msg: Buffer.from(JSON.stringify(vesting_msg), 'utf8').toString('base64'),
                                    // 베스팅 메시지를 base64로 인코딩
      }
    }
  );
  
  const send_tx = await wallet.createAndSignTx({
    msgs: [execute],
    gasPrices: { uluna: 0.015 },
  });
  const txres = await lcd.tx.broadcastSync(send_tx);
```


베스팅 물량 발생을 스케쥴하는 "vesting_schedule"은 3가지 종류.
- "linear_vesting": 선형적으로 증가하는 베스팅.
- "periodic_vesting": 특정 시간간격으로 발생하는 베스팅.
- "conditional_vesting": 조건이 만족하면 발생하는 베스팅.

### linear_vesting

시작 시간과 끝 시간, 그리고 총 물량을 설정.

```json
{
  "register_vesting_account": {
    ...
    "vesting_schedule": {
      "linear_vesting": {            // 선형 베스팅
        "start_time": "1650016200",  // 시작 시간(초), 2022-04-15 09:50:00 UTC
        "end_time": "1650016500",    // 끝 시간(초),   2022-04-15 09:55:00 UTC
        "vesting_amount": "600000"   // 베스팅할 전체 물량
      }
    }
  }
}
```

### periodic_vesting

시작 시간과 끝 시간, 그리고 시간 간격과 1회 발생 물량을 설정.
총 물량 = (({끝 시간} - {시작 시간}) / {시간 간격}) * {1회 발생 물량})

```json
{
  "register_vesting_account": {
    ...
    "vesting_schedule": {
      "periodic_vesting": {          // 주기적 베스팅
        "start_time": "1649911800",  // 시작 시간(초), 2022-04-14 04:50:00 UTC
        "end_time": "1649912100",    // 끝 시간(초),   2022-04-14 04:55:00 UTC
        "vesting_interval": "60",    // 발생 간격(초), 1분마다
        "amount": "100000"           // 1회 발생 물량
      }
    }
  }
}
```

### conditional_vesting

시작 시간과 끝 시간 간격 사이에 발생 조건 설정, 그리고 1회 발생 물량을 설정.

```json
{
  "register_vesting_account": {
    ...
    "vesting_schedule": {
      "conditional_vesting": {       // 조건부 베스팅
        "start_time": "1650343500",  // 시작 시간(초), 2022-04-19 04:45:00 UTC
        "end_time": "1650553199",    // 끝 시간(초),   2022-04-21 14:59:59 UTC
        "amount": "100000",          // 1회 발생 물량
        "condition": {               // 발생 조건
          "style": "daily",          // 매일마다
          "hour": 5                  // 5시에 발생
        }
      }
    }
  }
}
```

조건의 "style"은 4가지 종류
- "daily": 매일 특정 시간에 발생. ("hour" 속성 사용)
- "weekly": 매주 특정 요일에 발생. ("hour", "weekday" 속성 사용)
- "monthly": 매달 특정 날짜에 발생. ("hour", "day" 속성 사용)
- "yearly": 매년 특정 월에 발생. ("hour", "day", "month" 속성 사용)

설정할 수 있는 속성은 4가지
- "hour": UTC 기준 시간. 0 ~ 23. null -> 0.
- "weekday": 요일. 0(일) ~ 6(토). null -> 0.
- "day": 날짜. 1 ~ 31. null -> 1.
- "month": 월. 1 ~ 12. null -> 1.


## Execute deregister_vesting_account

베스팅 스케쥴 취소하기.
등록할 때 설정한 관리자 계정(master_address)을 사용해서 실행해야 함.

```json
{
  "deregister_vesting_account": {
    "address": "terra1...ylya",  // 받는 사람 주소
    "vesting_key": "test1"       // 취소할 베스팅 키
  }
}
```


## Execute claim

클레임 실행.
등록할 때 설정한 수신자 계정(address)을 사용해서 실행해야 함.

```json
{
  "claim": {
    "vesting_keys": [  // 클레임할 베스팅 키 목록
      "test1",
      "test2"
    ]
  }
}
```


## Query

등록된 베스팅 스케쥴 확인.

Input:
```json
{
  "vesting_account": {
    "address": "terra1...ylya"  // 수신자 주소
  }
}
```

Output:
```json
{
  "address": "terra1...ylya",  // 수신자 주소
  "vestings": [                // 베스팅 스케쥴 목록
    {
      "master_address": "terra1...0r7f",  // 관리자 주소
      "vesting_key": "test1",             // 베스팅 키
      "vesting_denom": {                  // 토큰 종류
        "native": "uluna"
      },
      "vesting_amount": "100000",  // 총 베스팅 물량
      "vested_amount": "100000",   // 현재까지 발생된 물량
      "vesting_schedule": {
        "linear_vesting": {
          "start_time": "1650379800",
          "end_time": "1650379801",
          "vesting_amount": "100000"
        }
      },
      "claimable_amount": "100000"  // 클레임 가능한 물량
    },
    {
      "master_address": "terra1...0r7f",
      "vesting_key": "test2",
      "vesting_denom": {
        "native": "uluna"
      },
      "vesting_amount": "600000",
      "vested_amount": "0",
      "vesting_schedule": {
        "periodic_vesting": {
          "start_time": "1650015000",
          "end_time": "1650015300",
          "vesting_interval": "60",
          "amount": "100000"
        }
      },
      "claimable_amount": "0"
    },
    {
      "master_address": "terra1...0r7f",
      "vesting_key": "test3",
      "vesting_denom": {
        "native": "uluna"
      },
      "vesting_amount": "300000",
      "vested_amount": "100000",
      "vesting_schedule": {
        "conditional_vesting": {
          "start_time": "1650343500",
          "end_time": "1650553199",
          "amount": "100000",
          "condition": {
            "style": "daily",
            "hour": 5,
            "weekday": null,
            "day": null,
            "month": null
          }
        }
      },
      "claimable_amount": "0"
    }
  ]
}
```

