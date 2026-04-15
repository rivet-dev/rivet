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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faComet.js
var require_faComet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faComet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "comet";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e003";
    var svgPathData = "M299.4 75.6L493.7 .9c1.5-.6 3.2-.9 4.8-.9C506 0 512 6 512 13.5c0 1.7-.3 3.3-.9 4.8L436.4 212.6l29.3-2.3c12.8-1 21.5 12.8 15.1 24L378.6 410.7C342.3 473.4 275.4 512 203 512C90.9 512 0 421.1 0 309c0-72.4 38.6-139.4 101.3-175.7L277.7 31.2c11.1-6.5 25 2.2 24 15.1l-2.3 29.3zM192 200c-6.1 0-11.7 3.5-14.3 8.9l-29.2 59.2-65.3 9.5c-6 .9-11 5.1-12.9 10.9s-.3 12.2 4 16.4l47.2 46-11.2 65c-1 6 1.4 12.1 6.4 15.7s11.5 4.1 16.8 1.2L192 402.1l58.4 30.7c5.4 2.8 11.9 2.4 16.8-1.2s7.4-9.6 6.4-15.7l-11.2-65 47.2-46c4.4-4.3 5.9-10.6 4-16.4s-6.9-10-12.9-10.9l-65.3-9.5-29.2-59.2c-2.7-5.5-8.3-8.9-14.3-8.9z";
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
    exports.faComet = exports.definition;
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

// src/_entries/faComet.js
var import_faComet = __toESM(require_faComet());
var export_faComet = import_faComet.definition;
export {
  export_faComet as faComet
};
