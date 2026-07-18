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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPenFancy.js
var require_faPenFancy = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPenFancy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pen-fancy";
    var width = 512;
    var height = 512;
    var aliases = [128395, 10002];
    var unicode = "f5ac";
    var svgPathData = "M373.5 27.1c15-17.2 36.7-27.1 59.6-27.1 43.6 0 79 35.4 79 79 0 22.8-9.9 44.6-27.1 59.6L283.7 313.8 273 303 209 239 198.2 228.3 373.5 27.1zM161.1 259C162 260 188 286 239 337l13.9 13.9-17.1 74.2c-3.9 17.1-16.9 30.7-33.8 35.4l-169.8 47.5 92.3-92.3c1.2 .1 2.3 .2 3.5 .2 17.7 0 32-14.3 32-32s-14.3-32-32-32-32 14.3-32 32c0 1.2 .1 2.4 .2 3.5L3.9 479.8 51.5 310c4.7-16.9 18.3-29.9 35.4-33.8L161.1 259z";
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
    exports.faPenFancy = exports.definition;
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

// src/_entries/faPenFancy.js
var import_faPenFancy = __toESM(require_faPenFancy());
var export_faPenFancy = import_faPenFancy.definition;
export {
  export_faPenFancy as faPenFancy
};
