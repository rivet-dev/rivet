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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWatch.js
var require_faWatch = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWatch.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "watch";
    var width = 384;
    var height = 512;
    var aliases = [8986];
    var unicode = "f2e1";
    var svgPathData = "M112 0L272 0c26.5 0 48 21.5 48 48l0 24.2C283.7 46.8 239.6 32 192 32s-91.7 14.8-128 40.2L64 48C64 21.5 85.5 0 112 0zM64 464l0-24.2c36.3 25.3 80.4 40.2 128 40.2s91.7-14.8 128-40.2l0 24.2c0 26.5-21.5 48-48 48l-160 0c-26.5 0-48-21.5-48-48zM0 256a192 192 0 1 1 384 0A192 192 0 1 1 0 256zM192 152c-13.3 0-24 10.7-24 24l0 80c0 8 4 15.5 10.7 20l48 32c11 7.4 25.9 4.4 33.3-6.7s4.4-25.9-6.7-33.3L216 243.2l0-67.2c0-13.3-10.7-24-24-24z";
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
    exports.faWatch = exports.definition;
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

// src/_entries/faWatch.js
var import_faWatch = __toESM(require_faWatch());
var export_faWatch = import_faWatch.definition;
export {
  export_faWatch as faWatch
};
