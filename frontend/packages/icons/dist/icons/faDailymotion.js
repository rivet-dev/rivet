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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDailymotion.js
var require_faDailymotion = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDailymotion.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "dailymotion";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e052";
    var svgPathData = "M298.9 267c-7.4-4.2-15.8-6.3-24.4-6.2-13.2 0-24.4 4.4-33.4 13.3s-13.6 20-13.6 33.4c0 14.1 4.4 25.6 13.3 34.6s20 13.4 33.4 13.4c13.7 0 25-4.6 34.1-13.8S322 321.1 322 307.5c0-8.2-2.1-16.3-6.1-23.5-4-7.1-9.8-13-17-17zM0 32l0 448 448 0 0-448-448 0zM374.7 405.3l-53.1 0 0-23.9-.7 0c-10.5 17.5-29.1 26.2-55.8 26.2-18.4 0-34.7-4.4-48.9-13.1-14.1-8.6-25.5-21-32.9-35.8-7.7-15.1-11.6-32.1-11.6-50.9 0-18.4 3.9-35.1 11.8-50.2 7.5-14.7 18.9-27.1 32.9-35.8 14.1-8.7 30-13.1 47.7-13.1 10.2-.1 20.2 1.6 29.7 5.2 8.9 3.5 17.2 9.1 25 17l0-77.9 55.8-12.1 0 264.4z";
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
    exports.faDailymotion = exports.definition;
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

// src/_entries/faDailymotion.js
var import_faDailymotion = __toESM(require_faDailymotion());
var export_faDailymotion = import_faDailymotion.definition;
export {
  export_faDailymotion as faDailymotion
};
