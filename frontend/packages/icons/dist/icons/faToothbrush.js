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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faToothbrush.js
var require_faToothbrush = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faToothbrush.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "toothbrush";
    var width = 576;
    var height = 512;
    var aliases = [129701];
    var unicode = "f635";
    var svgPathData = "M64 224c-35.3 0-64-28.7-64-64s28.7-64 64-64l224 0c31.2 0 38.2-22.7 39.6-48c.5-8.8 7.6-16.1 16.4-15.2c52.4 5.3 104 36.7 104 95.2c0 53-43 96-96 96L64 224zm-8 32c13.3 0 24 10.7 24 24l0 136 32 0 0-136c0-13.3 10.7-24 24-24s24 10.7 24 24l0 136 32 0 0-136c0-13.3 10.7-24 24-24s24 10.7 24 24l0 136 32 0 0-136c0-13.3 10.7-24 24-24s24 10.7 24 24l0 136 32 0 0-136c0-13.3 10.7-24 24-24s24 10.7 24 24l0 136 144 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 480c-17.7 0-32-14.3-32-32s14.3-32 32-32l0-136c0-13.3 10.7-24 24-24z";
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
    exports.faToothbrush = exports.definition;
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

// src/_entries/faToothbrush.js
var import_faToothbrush = __toESM(require_faToothbrush());
var export_faToothbrush = import_faToothbrush.definition;
export {
  export_faToothbrush as faToothbrush
};
