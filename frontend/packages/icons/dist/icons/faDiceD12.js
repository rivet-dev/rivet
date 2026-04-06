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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDiceD12.js
var require_faDiceD12 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDiceD12.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dice-d12";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6ce";
    var svgPathData = "M318.2 0c4.2 0 8.4 1.1 12 3.2l72 41.6c5.9 3.4 5.1 12.2-1.3 14.5L264.1 108.1c-5.2 1.9-10.9 1.9-16.1 0L111.1 59.3c-6.4-2.3-7.2-11.1-1.3-14.5l72-41.6c3.6-2.1 7.8-3.2 12-3.2L318.2 0zM149.9 490.4L74.2 446.6c-3.6-2.1-6.7-5.1-8.8-8.8L3.2 330.2c-2.1-3.6-3.2-7.8-3.2-12l0-84.2c0-7.1 8.6-10.7 13.7-5.7l96.1 96.1c2.7 2.7 4.8 6.1 5.9 9.8L161.6 481c2.2 7-5.3 13-11.6 9.3zM512 233.9l0 84.2c0 4.2-1.1 8.4-3.2 12L446.6 437.8c-2.1 3.6-5.1 6.7-8.8 8.8l-75.8 43.7c-6.3 3.6-13.8-2.4-11.6-9.3l45.9-146.9c1.2-3.7 3.2-7.1 5.9-9.8l96.1-96.1c5-5 13.7-1.5 13.7 5.7zM15.3 184.7c-5.1-5.1-6.2-13-2.5-19.3L57.4 88c3.9-6.7 12-9.7 19.2-7.1l152.8 54.6c6.4 2.3 10.6 8.3 10.6 15.1l0 85.7c0 6.1-3.4 11.6-8.8 14.3l-89.7 44.8c-6.2 3.1-13.6 1.9-18.5-3L15.3 184.7zM454.6 88l44.7 77.3c3.6 6.3 2.6 14.2-2.5 19.3L389 292.4c-4.9 4.9-12.3 6.1-18.5 3l-89.7-44.8c-5.4-2.7-8.8-8.3-8.8-14.3l0-85.7c0-6.8 4.3-12.8 10.6-15.1L435.4 80.9c7.3-2.6 15.4 .4 19.2 7.1zM295.5 512l-78.9 0c-7 0-13.2-4.5-15.3-11.2L151.4 341.2c-2.3-7.5 1.1-15.6 8.1-19.1l89.3-44.7c4.5-2.3 9.8-2.3 14.3 0l89.3 44.7c7 3.5 10.5 11.6 8.1 19.1L310.7 500.8c-2.1 6.7-8.3 11.2-15.3 11.2z";
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
    exports.faDiceD12 = exports.definition;
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

// src/_entries/faDiceD12.js
var import_faDiceD12 = __toESM(require_faDiceD12());
var export_faDiceD12 = import_faDiceD12.definition;
export {
  export_faDiceD12 as faDiceD12
};
