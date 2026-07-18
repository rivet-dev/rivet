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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonDigging.js
var require_faPersonDigging = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonDigging.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-digging";
    var width = 576;
    var height = 512;
    var aliases = ["digging"];
    var unicode = "f85e";
    var svgPathData = "M208 40a56 56 0 1 1 112 0 56 56 0 1 1 -112 0zM10.5 181.3c5.9-11.9 20.3-16.7 32.2-10.7l24.6 12.3 12.2-20.4c18.9-31.5 53.2-50.5 89.6-50.5 46.2 0 87.7 30.5 100.5 75.4l32.2 112.7 92.9 46.4 25.8-43c5.8-9.6 16.2-15.5 27.4-15.5s21.7 5.9 27.4 15.5l96 160c5.9 9.9 6.1 22.2 .4 32.2S555.5 512 544 512l-192 0c-11.5 0-22.2-6.2-27.8-16.2s-5.5-22.3 .4-32.2L370 387.8 21.3 213.5c-11.9-5.9-16.7-20.3-10.7-32.2zM94.3 307.4l112 56c10.8 5.4 17.7 16.5 17.7 28.6l0 88c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-68.2-61.3-30.7-36.3 109c-5.6 16.8-23.7 25.8-40.5 20.2S-3.9 486.6 1.7 469.9l48-144c2.9-8.8 9.5-15.9 18.1-19.4s18.3-3.2 26.6 .9z";
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
    exports.faPersonDigging = exports.definition;
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

// src/_entries/faPersonDigging.js
var import_faPersonDigging = __toESM(require_faPersonDigging());
var export_faPersonDigging = import_faPersonDigging.definition;
export {
  export_faPersonDigging as faPersonDigging
};
