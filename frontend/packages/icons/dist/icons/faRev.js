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

// src/node_modules/@fortawesome/free-brands-svg-icons/faRev.js
var require_faRev = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faRev.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "rev";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f5b2";
    var svgPathData = "M158.7 274.9a65.6 65.6 0 1 1 131.2 0 65.6 65.6 0 1 1 -131.2 0zm270.7-5.1l-.1 0c-.7-29.3-7.7-58.2-20.5-84.6s-31.2-49.7-53.8-68.4L309.6 143c22 14.2 40.1 33.8 52.7 56.8s19.1 48.8 19.1 75.1c0 86.6-70.5 157.1-157.1 157.1S67.2 361.5 67.2 274.9c0-81.9 63-149.3 143-156.4l0 39.1 108.8-62.8-108.8-62.8 0 38.3c-106.7 7.2-191 96-191 204.6 0 111.6 89.1 202.3 200.1 205l0 .1 210.2 0 0-210.2z";
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
    exports.faRev = exports.definition;
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

// src/_entries/faRev.js
var import_faRev = __toESM(require_faRev());
var export_faRev = import_faRev.definition;
export {
  export_faRev as faRev
};
