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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFunction.js
var require_faFunction = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFunction.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "function";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f661";
    var svgPathData = "M64 96c0-53 43-96 96-96l32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0c-17.7 0-32 14.3-32 32l0 96 32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0 0 127.5c0 48.4-36.1 89.3-84.1 95.3l-7.9 1C18.4 481.9 2.4 469.5 .2 452s10.2-33.5 27.8-35.7l7.9-1c16-2 28-15.6 28-31.8L64 256l-32 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l32 0 0-96zm266.7 81.7C303.7 218.4 288 267.3 288 320s15.7 101.6 42.7 142.3c9.8 14.7 5.7 34.6-9 44.3s-34.6 5.7-44.3-9C243.6 446.7 224 385.6 224 320s19.6-126.7 53.3-177.7c9.8-14.7 29.6-18.8 44.3-9s18.8 29.6 9 44.3zm202.6 0c-9.8-14.7-5.7-34.6 9-44.3s34.6-5.7 44.3 9C620.4 193.3 640 254.4 640 320s-19.6 126.7-53.3 177.7c-9.8 14.7-29.6 18.8-44.3 9s-18.8-29.6-9-44.3c27-40.7 42.7-89.6 42.7-142.3s-15.7-101.6-42.7-142.3zM398.6 241.4L432 274.7l33.4-33.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3L477.3 320l33.4 33.4c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0L432 365.3l-33.4 33.4c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L386.7 320l-33.4-33.4c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0z";
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
    exports.faFunction = exports.definition;
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

// src/_entries/faFunction.js
var import_faFunction = __toESM(require_faFunction());
var export_faFunction = import_faFunction.definition;
export {
  export_faFunction as faFunction
};
