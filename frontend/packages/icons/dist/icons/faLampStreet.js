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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLampStreet.js
var require_faLampStreet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLampStreet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lamp-street";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e1c5";
    var svgPathData = "M64 176c0-61.9 50.1-112 112-112c57.4 0 104.7 43.2 111.2 98.8C207.1 176.6 144 239.9 130.6 320.1C127.7 337.6 142.3 352 160 352l320 0c17.7 0 32.3-14.4 29.4-31.9c-13.5-80.6-77.2-144.2-157.9-157.6C344.6 71.6 268.7 0 176 0C78.8 0 0 78.8 0 176L0 480c0 17.7 14.3 32 32 32s32-14.3 32-32l0-304zM320 448c35.3 0 64-28.7 64-64l-128 0c0 35.3 28.7 64 64 64z";
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
    exports.faLampStreet = exports.definition;
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

// src/_entries/faLampStreet.js
var import_faLampStreet = __toESM(require_faLampStreet());
var export_faLampStreet = import_faLampStreet.definition;
export {
  export_faLampStreet as faLampStreet
};
