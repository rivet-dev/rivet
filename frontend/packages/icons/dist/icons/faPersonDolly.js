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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonDolly.js
var require_faPersonDolly = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonDolly.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-dolly";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f4d0";
    var svgPathData = "M32 48a48 48 0 1 1 96 0A48 48 0 1 1 32 48zM96 241.7l0 65.1 51 51c7.1 7.1 11.8 16.2 13.4 26.1l15.2 90.9c2.9 17.4-8.9 33.9-26.3 36.8s-33.9-8.9-36.8-26.3L98.1 399.4 18.7 320C6.7 308 0 291.7 0 274.7l0-88.1C0 154.2 26.2 128 58.6 128c24.1 0 46.5 12 59.9 32l42.6 64 46.1 0-6.5-26.2c-3.2-12.9 4.6-25.9 17.5-29.1s25.9 4.6 29.1 17.5l49.6 198.4c17.6 2.4 32.9 12 42.8 25.7l134.4-33.6c12.9-3.2 25.9 4.6 29.1 17.5s-4.6 25.9-17.5 29.1L351.4 456.9C347.1 488 320.3 512 288 512c-35.3 0-64-28.7-64-64c0-21.3 10.4-40.1 26.3-51.7L223.3 288l-70.7 0c-16 0-31-8-39.9-21.4L96 241.7zM0 480L0 346.5l64 64L64 480s0 0 0 0c0 17.7-14.3 32-32 32s-32-14.3-32-32zm288-16a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm69.4-87.8c-17.1 4.3-34.5-6.1-38.8-23.3l-22.9-91.5c-4.3-17.1 6.1-34.5 23.3-38.8l91.5-22.9c17.1-4.3 34.5 6.1 38.8 23.3l22.9 91.5c4.3 17.1-6.1 34.5-23.3 38.8l-91.5 22.9z";
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
    exports.faPersonDolly = exports.definition;
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

// src/_entries/faPersonDolly.js
var import_faPersonDolly = __toESM(require_faPersonDolly());
var export_faPersonDolly = import_faPersonDolly.definition;
export {
  export_faPersonDolly as faPersonDolly
};
