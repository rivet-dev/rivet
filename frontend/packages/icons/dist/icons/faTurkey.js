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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTurkey.js
var require_faTurkey = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTurkey.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "turkey";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f725";
    var svgPathData = "M472 56c0 22.1-1.2 49.3-19.3 62l-31.6 22.3C375.9 112.2 327.9 96 288 96C176 96 0 224 0 384C0 512 160 512 288 512s288 0 288-128c0-63-27.3-121.1-67.3-168.3c-.4 .2-.7 .4-1 .6l-28.3 20c-8.3 5.9-15.1 15-21.2 28.3c-6.1 13.4-10.5 28.6-15.4 46l-.3 1c-7.9 27.5-24 53-47.9 72.3c-61.8 50-152.5 40.5-202.5-21.4c-13.9-17.2-23.2-36.6-28.1-56.7c-2.3-9.4 5.3-17.9 15-17.9c8 0 14.6 5.8 16.9 13.5c4 14.5 10.9 28.5 21 41c21.3 26.3 52.1 40.5 83.5 41.5c25.9 .8 52.2-7.3 74-24.9c18.6-15 31.1-34.8 37.2-56.2l.6-2c9.6-33.7 20.1-70.5 48.7-90.6L486.7 192s0 0 0 0l2.5-1.8c18.3-12.9 43.3-10.4 67.4-8c9.4 .9 18.6 1.8 27.3 1.8c30.9 0 56-25.1 56-56s-25.1-56-56-56c-1.1 0-2-1.1-1.7-2.2c1.1-4.4 1.7-9 1.7-13.8c0-30.9-25.1-56-56-56s-56 25.1-56 56z";
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
    exports.faTurkey = exports.definition;
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

// src/_entries/faTurkey.js
var import_faTurkey = __toESM(require_faTurkey());
var export_faTurkey = import_faTurkey.definition;
export {
  export_faTurkey as faTurkey
};
