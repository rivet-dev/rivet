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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle5.js
var require_faCircle5 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircle5.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-5";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0f2";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM208 128l104 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-84 0-7.8 43 61.4 6.9c40.1 4.5 70.4 38.4 70.4 78.8c0 43.8-35.5 79.3-79.3 79.3l-36 0c-27.5 0-53.3-13.7-68.6-36.6l-4.1-6.1c-7.4-11-4.5-25.9 6.5-33.3s25.9-4.5 33.3 6.5l4.1 6.1c6.4 9.6 17.2 15.3 28.8 15.3l36 0c17.3 0 31.3-14 31.3-31.3c0-15.9-12-29.3-27.8-31.1l-86.9-9.8c-6.6-.7-12.6-4.2-16.6-9.5s-5.5-12.1-4.4-18.6l16-88c2.1-11.4 12-19.7 23.6-19.7z";
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
    exports.faCircle5 = exports.definition;
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

// src/_entries/faCircle5.js
var import_faCircle5 = __toESM(require_faCircle5());
var export_faCircle5 = import_faCircle5.definition;
export {
  export_faCircle5 as faCircle5
};
