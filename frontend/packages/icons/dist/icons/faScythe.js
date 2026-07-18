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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScythe.js
var require_faScythe = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScythe.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scythe";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f710";
    var svgPathData = "M614.7 .7c5.6 1.2 10.5 3.8 14.5 7.3c8.4 7.4 12.6 19 10.1 30.7l-4.8 22.6L577.6 326.7l-34.3 160c-3.7 17.3-20.7 28.3-38 24.6s-28.3-20.7-24.6-38l26-121.3L416 352c-17.7 0-32-14.3-32-32s14.3-32 32-32l104.4 0L569 61.3l7.7-36s0 0 0 0L582.1 0 608 0c2.2 0 4.5 .2 6.7 .7zM352 0L549.4 0l-4 18.6L508.3 192 12.7 192C5.7 192 0 186.3 0 179.2c0-2.1 .5-4.1 1.6-6C15 151.1 114.8 0 352 0z";
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
    exports.faScythe = exports.definition;
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

// src/_entries/faScythe.js
var import_faScythe = __toESM(require_faScythe());
var export_faScythe = import_faScythe.definition;
export {
  export_faScythe as faScythe
};
