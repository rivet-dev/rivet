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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHatChef.js
var require_faHatChef = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHatChef.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hat-chef";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f86b";
    var svgPathData = "M153.6 51.2C177 20.1 214.1 0 256 0s79 20.1 102.4 51.2c16-12.1 36-19.2 57.6-19.2c7.1 0 14.4 .8 21.6 2.5C482 44.7 512 84.3 512 128c0 7.1-.8 14.4-2.5 21.6C496 224 448 320 448 320l-74.4 0 26.2-157.4c1.5-8.7-4.4-17-13.2-18.4s-17 4.4-18.4 13.2L341.1 320 272 320l0-160c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 160-69.1 0L143.8 157.4c-1.5-8.7-9.7-14.6-18.4-13.2s-14.6 9.7-13.2 18.4L138.4 320 64 320s-48-96-61.5-170.4C.8 142.4 0 135.1 0 128c0-43.7 30-83.3 74.4-93.5C81.6 32.8 88.9 32 96 32c21.6 0 41.6 7.1 57.6 19.2zM448 352l0 96c0 35.3-28.7 64-64 64l-256 0c-35.3 0-64-28.7-64-64l0-96 384 0z";
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
    exports.faHatChef = exports.definition;
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

// src/_entries/faHatChef.js
var import_faHatChef = __toESM(require_faHatChef());
var export_faHatChef = import_faHatChef.definition;
export {
  export_faHatChef as faHatChef
};
