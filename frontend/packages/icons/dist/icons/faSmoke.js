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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSmoke.js
var require_faSmoke = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSmoke.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "smoke";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f760";
    var svgPathData = "M172.3 194.3c-9.2-1.5-18.7-2.3-28.3-2.3c-29.2 0-56.7 7.1-80.9 19.6C43.7 189.2 32 160 32 128C32 57.3 89.3 0 160 0c40.1 0 75.9 18.4 99.3 47.3C277.4 37.5 298 32 320 32c44.3 0 83.4 22.5 106.4 56.8c-11.5 7.3-22 16-31.2 25.9C372.7 102.8 347.2 96 320 96c-66.5 0-123.5 40.6-147.7 98.3zm276.3 62.5C419.5 217.5 372.7 192 320 192c-31.5 0-60.9 9.1-85.7 24.9c-9.6-5.8-19.8-10.6-30.5-14.5C224 158.5 268.4 128 320 128c31.3 0 60.1 11.3 82.3 30c22.4-37.1 63.1-62 109.7-62c70.7 0 128 57.3 128 128c0 42.2-20.4 79.6-51.9 102.9C563.1 284.5 516.9 256 464 256c-5.2 0-10.3 .3-15.4 .8zM144 512C64.5 512 0 447.5 0 368s64.5-144 144-144c34.5 0 66.1 12.1 90.9 32.3C257.6 236.2 287.4 224 320 224c49.1 0 91.8 27.7 113.2 68.3c9.8-2.8 20.1-4.3 30.8-4.3c61.9 0 112 50.1 112 112s-50.1 112-112 112l-320 0z";
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
    exports.faSmoke = exports.definition;
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

// src/_entries/faSmoke.js
var import_faSmoke = __toESM(require_faSmoke());
var export_faSmoke = import_faSmoke.definition;
export {
  export_faSmoke as faSmoke
};
