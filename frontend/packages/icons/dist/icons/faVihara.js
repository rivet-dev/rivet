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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVihara.js
var require_faVihara = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVihara.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "vihara";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f6a7";
    var svgPathData = "M273.6-3.2c8.5-6.4 20.3-6.4 28.8 0L443.2 102.4c8.3 6.2 18.4 9.6 28.8 9.6 13.3 0 24 10.7 24 24s-10.7 24-24 24l-24 0 0 48 58.7 64 13.3 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-8 0 0 48 24.4 32 15.6 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-40 0 0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32-128 0 0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32-128 0 0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32-40 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l22.1 0 17.9-32 0-48-8 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l13.3 0 58.7-64 0-48-24 0c-13.3 0-24-10.7-24-24s10.7-24 24-24c10.4 0 20.5-3.4 28.8-9.6L273.6-3.2zM128 368l320 0 0-48-320 0 0 48zM384 160l-192 0 0 48 192 0 0-48z";
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
    exports.faVihara = exports.definition;
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

// src/_entries/faVihara.js
var import_faVihara = __toESM(require_faVihara());
var export_faVihara = import_faVihara.definition;
export {
  export_faVihara as faVihara
};
