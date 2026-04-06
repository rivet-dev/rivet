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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonDress.js
var require_faPersonDress = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonDress.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-dress";
    var width = 384;
    var height = 512;
    var aliases = ["female"];
    var unicode = "f182";
    var svgPathData = "M136 24a56 56 0 1 1 112 0 56 56 0 1 1 -112 0zM112 384l-25.8 0c-10.9 0-18.6-10.7-15.2-21.1l43-129-48.3 65.1c-10.5 14.2-30.6 17.2-44.8 6.6s-17.2-30.6-6.6-44.8l70.5-95C110 132 149.7 112 192 112s82 20 107.2 53.9l70.5 95c10.5 14.2 7.6 34.2-6.6 44.8s-34.2 7.6-44.8-6.6L270 234 313 362.9c3.5 10.4-4.3 21.1-15.2 21.1l-25.8 0 0 128c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-128-32 0 0 128c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-128z";
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
    exports.faPersonDress = exports.definition;
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

// src/_entries/faFemale.js
var import_faPersonDress = __toESM(require_faPersonDress());
var export_faFemale = import_faPersonDress.definition;
export {
  export_faFemale as faFemale
};
