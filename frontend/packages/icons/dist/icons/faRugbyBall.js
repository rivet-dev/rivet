var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRugbyBall.js
var require_faRugbyBall = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRugbyBall.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rugby-ball";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3c6";
    var svgPathData = "M368 16c13.4 0 26.4 2.1 38.5 5.9c-21.1 1.8-44.5 8.7-69 19.5c-53.4 23.5-114.8 66.9-172.1 124.1S64.9 284.1 41.4 337.6c-10.8 24.5-17.7 47.9-19.5 69C18.1 394.4 16 381.4 16 368l0-32C16 159.3 159.3 16 336 16l32 0zm122.1 89.5c3.8 12.2 5.9 25.1 5.9 38.5l0 32c0 176.7-143.3 320-320 320l-32 0c-13.4 0-26.4-2.1-38.5-5.9c21.1-1.8 44.5-8.7 69-19.5c53.4-23.5 114.8-66.9 172.1-124.1s100.6-118.6 124.1-172.1c10.8-24.5 17.7-47.9 19.5-69zM63.7 448.3c-6.7-6.7-10.9-17.8-10.2-34.8c.7-17.1 6.3-38.5 17.1-63c21.5-49 62.3-107.1 117.5-162.3s113.4-95.9 162.3-117.5c24.5-10.8 45.9-16.4 63-17.1c17.1-.7 28.1 3.5 34.8 10.2s10.9 17.8 10.2 34.8c-.7 17.1-6.3 38.5-17.1 63c-21.5 49-62.3 107.1-117.5 162.3s-113.4 95.9-162.3 117.5c-24.5 10.8-45.9 16.4-63 17.1c-17.1 .7-28.1-3.4-34.8-10.2z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faRugbyBall = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faRugbyBall.js
var import_faRugbyBall = __toESM(require_faRugbyBall());
var export_faRugbyBall = import_faRugbyBall.definition;
export {
  export_faRugbyBall as faRugbyBall
};
