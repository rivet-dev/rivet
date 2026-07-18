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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDiceD4.js
var require_faDiceD4 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDiceD4.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dice-d4";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6d0";
    var svgPathData = "M228.8 .7C235.5 2.8 240 9 240 16l0 480c0 6.3-3.7 12-9.4 14.6s-12.5 1.6-17.2-2.6L5.4 324c-6.1-5.4-7.2-14.5-2.5-21.2l208-296c4-5.7 11.3-8.2 17.9-6.1zm54.4 0c6.7-2.1 13.9 .3 17.9 6.1l208 296c4.7 6.7 3.6 15.8-2.5 21.2l-208 184c-4.7 4.2-11.4 5.2-17.2 2.6s-9.4-8.3-9.4-14.6l0-480c0-7 4.5-13.2 11.2-15.3z";
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
    exports.faDiceD4 = exports.definition;
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

// src/_entries/faDiceD4.js
var import_faDiceD4 = __toESM(require_faDiceD4());
var export_faDiceD4 = import_faDiceD4.definition;
export {
  export_faDiceD4 as faDiceD4
};
