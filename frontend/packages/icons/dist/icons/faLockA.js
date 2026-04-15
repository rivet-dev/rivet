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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLockA.js
var require_faLockA = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLockA.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lock-a";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e422";
    var svgPathData = "M224 64c44.2 0 80 35.8 80 80l0 48-160 0 0-48c0-44.2 35.8-80 80-80zM80 144l0 48-16 0c-35.3 0-64 28.7-64 64L0 448c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-192c0-35.3-28.7-64-64-64l-16 0 0-48C368 64.5 303.5 0 224 0S80 64.5 80 144zm49.7 280.8L209.6 265c.8-1.6 1.8-3 3-4.3c1.4-1.5 3.1-2.6 5-3.4c2.1-.9 4.2-1.3 6.4-1.3s4.3 .4 6.4 1.3c1.8 .8 3.5 1.9 5 3.4c1.3 1.3 2.3 2.7 3.1 4.3l79.9 159.9c4 7.9 .7 17.5-7.2 21.5s-17.5 .7-21.5-7.2L270.1 400l-92.2 0-19.6 39.2c-4 7.9-13.6 11.1-21.5 7.2s-11.1-13.6-7.2-21.5zM193.9 368l60.2 0L224 307.8 193.9 368z";
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
    exports.faLockA = exports.definition;
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

// src/_entries/faLockA.js
var import_faLockA = __toESM(require_faLockA());
var export_faLockA = import_faLockA.definition;
export {
  export_faLockA as faLockA
};
