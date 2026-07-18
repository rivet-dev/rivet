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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTennisBall.js
var require_faTennisBall = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTennisBall.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tennis-ball";
    var width = 512;
    var height = 512;
    var aliases = [127934];
    var unicode = "f45e";
    var svgPathData = "M.2 245.1C.1 248.7 0 252.3 0 256C0 397.4 114.6 512 256 512c3.7 0 7.3-.1 10.9-.2c-3.8-10.7-6.6-21.8-8.4-33.1c-4.4-27.2-2.9-55 4.4-81.6s20.2-51.2 37.9-72.4s39.7-38.2 64.5-50.1s52-18.2 79.5-18.7c22.8-.4 45.5 3.3 67 10.9c.2-3.6 .2-7.2 .2-10.9C512 114.6 397.4 0 256 0c-3.7 0-7.3 .1-10.9 .2c10.7 30.5 13.7 63.4 8.2 95.6c-7.1 42.4-28.3 81.2-60 110.1s-72.4 46.4-115.2 49.6c-26.6 2-53.1-1.7-77.9-10.4zM206.8 4.7C104.9 24.6 24.6 104.9 4.7 206.8C26.9 216 51.1 220 75.3 218.2c34.5-2.5 67.2-16.6 92.8-39.9s42.6-54.5 48.4-88.7c4.8-28.7 1.4-58.2-9.7-84.9zM445.4 293.4c-22.2 .4-44 5.5-64 15.1s-37.7 23.3-52 40.3s-24.6 36.9-30.5 58.3s-7.1 43.8-3.5 65.7c1.9 11.9 5.2 23.5 9.8 34.6c101.9-19.8 182.2-100.2 202.1-202.1c-19.6-8.1-40.6-12.1-61.9-11.8z";
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
    exports.faTennisBall = exports.definition;
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

// src/_entries/faTennisBall.js
var import_faTennisBall = __toESM(require_faTennisBall());
var export_faTennisBall = import_faTennisBall.definition;
export {
  export_faTennisBall as faTennisBall
};
