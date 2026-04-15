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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPopcorn.js
var require_faPopcorn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPopcorn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "popcorn";
    var width = 448;
    var height = 512;
    var aliases = [127871];
    var unicode = "f819";
    var svgPathData = "M32.2 160c-.1-1.3-.2-2.7-.2-4c0-20.9 14.5-38.3 34-42.9c-1.3-4.2-2-8.6-2-13.1c0-24.3 19.7-44 44-44c1.9 0 3.7 .1 5.6 .3C118.7 37.7 135.7 24 156 24c8.5 0 16.5 2.4 23.2 6.6C186.2 12.7 203.6 0 224 0s37.8 12.7 44.8 30.6c6.7-4.2 14.7-6.6 23.2-6.6c20.3 0 37.3 13.7 42.4 32.3c1.8-.2 3.7-.3 5.6-.3c24.3 0 44 19.7 44 44c0 4.6-.7 9-2 13.1c19.5 4.5 34 22 34 42.9c0 1.3-.1 2.7-.2 4l-87.6 0-208.4 0-87.6 0zm-4 32l69.6 0 30.3 257.9c1 8.8 9 15.1 17.8 14s15.1-9 14-17.8L130 192l78 0 0 256c0 8.8 7.2 16 16 16s16-7.2 16-16l0-256 78 0L288.1 446.1c-1 8.8 5.2 16.7 14 17.8s16.7-5.2 17.8-14L350.2 192l69.6 0c15.6 0 28.2 12.6 28.2 28.2c0 2.5-.3 5.1-1 7.5L377.8 476.8C372 497.6 353.1 512 331.5 512l-215 0c-21.6 0-40.5-14.4-46.2-35.2L1 227.7c-.7-2.5-1-5-1-7.5C0 204.6 12.6 192 28.2 192z";
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
    exports.faPopcorn = exports.definition;
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

// src/_entries/faPopcorn.js
var import_faPopcorn = __toESM(require_faPopcorn());
var export_faPopcorn = import_faPopcorn.definition;
export {
  export_faPopcorn as faPopcorn
};
