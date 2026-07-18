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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTreeLarge.js
var require_faTreeLarge = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTreeLarge.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tree-large";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f7dd";
    var svgPathData = "M62 169.4L210.6 5.9C214 2.1 218.9 0 224 0s10 2.1 13.4 5.9L386 169.4c3.9 4.2 6 9.8 6 15.5c0 12.7-10.3 23.1-23.1 23.1H344l75.5 105.7c2.9 4.1 4.5 9 4.5 14.1c0 13.4-10.9 24.2-24.2 24.2H368l76.2 122c2.5 4 3.8 8.5 3.8 13.2c0 13.7-11.1 24.9-24.9 24.9H24.9C11.1 512 0 500.9 0 487.1c0-4.7 1.3-9.2 3.8-13.2L80 352H48.2C34.8 352 24 341.1 24 327.8c0-5 1.6-10 4.5-14.1L104 208H79.1C66.3 208 56 197.7 56 184.9c0-5.7 2.1-11.3 6-15.5z";
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
    exports.faTreeLarge = exports.definition;
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

// src/_entries/faTreeLarge.js
var import_faTreeLarge = __toESM(require_faTreeLarge());
var export_faTreeLarge = import_faTreeLarge.definition;
export {
  export_faTreeLarge as faTreeLarge
};
