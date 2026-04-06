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

// src/node_modules/@fortawesome/free-solid-svg-icons/faToiletsPortable.js
var require_faToiletsPortable = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faToiletsPortable.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "toilets-portable";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e584";
    var svgPathData = "M64 0l96 0c35.3 0 64 28.7 64 64l0 16-224 0 0-16C0 28.7 28.7 0 64 0zM0 128l224 0 0 360c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-8-128 0 0 8c0 13.3-10.7 24-24 24S0 501.3 0 488L0 128zM128 248l0 48c0 13.3 10.7 24 24 24l24 0 0-96-24 0c-13.3 0-24 10.7-24 24zM352 0l96 0c35.3 0 64 28.7 64 64l0 16-224 0 0-16c0-35.3 28.7-64 64-64zM288 128l224 0 0 360c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-8-128 0 0 8c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-360zM416 248l0 48c0 13.3 10.7 24 24 24l24 0 0-96-24 0c-13.3 0-24 10.7-24 24z";
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
    exports.faToiletsPortable = exports.definition;
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

// src/_entries/faToiletsPortable.js
var import_faToiletsPortable = __toESM(require_faToiletsPortable());
var export_faToiletsPortable = import_faToiletsPortable.definition;
export {
  export_faToiletsPortable as faToiletsPortable
};
