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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHouseSignal.js
var require_faHouseSignal = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHouseSignal.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "house-signal";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e012";
    var svgPathData = "M515.9 240l-19.9 0 0 144c0 35.3-28.7 64-64 64l-137 0c-24-91.7-93.5-165-183-194.3l0-13.7-19.9 0c-15.5 0-28.1-12.6-28.1-28.1 0-7.6 3.1-14.9 8.6-20.2L276.5-4.9C283.9-12 293.7-16 304-16s20.1 4 27.5 11.1L535.4 191.7c5.5 5.3 8.6 12.6 8.6 20.2 0 15.5-12.6 28.1-28.1 28.1zM280 192c-13.3 0-24 10.7-24 24l0 48c0 13.3 10.7 24 24 24l48 0c13.3 0 24-10.7 24-24l0-48c0-13.3-10.7-24-24-24l-48 0zM24 288c128.1 0 232 103.9 232 232 0 13.3-10.7 24-24 24s-24-10.7-24-24c0-101.6-82.4-184-184-184-13.3 0-24-10.7-24-24s10.7-24 24-24zm8 192a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM0 408c0-13.3 10.7-24 24-24 75.1 0 136 60.9 136 136 0 13.3-10.7 24-24 24s-24-10.7-24-24c0-48.6-39.4-88-88-88-13.3 0-24-10.7-24-24z";
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
    exports.faHouseSignal = exports.definition;
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

// src/_entries/faHouseSignal.js
var import_faHouseSignal = __toESM(require_faHouseSignal());
var export_faHouseSignal = import_faHouseSignal.definition;
export {
  export_faHouseSignal as faHouseSignal
};
