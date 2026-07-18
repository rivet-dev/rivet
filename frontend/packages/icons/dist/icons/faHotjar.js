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

// src/node_modules/@fortawesome/free-brands-svg-icons/faHotjar.js
var require_faHotjar = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faHotjar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "hotjar";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f3b1";
    var svgPathData = "M361.8 0c0 131.6-80.7 176.8-140.2 209.4-.6 .3-1.1 .6-1.6 .9-53.8 30.2-88.7 49.8-89.6 122l-98.1 0c0-131.5 80.7-176.7 140.2-209.3 54.8-29.8 90.3-50 90.3-123l99 0zM301.3 302.6c54.8-29.8 90.3-50 90.3-123l98 0c0 131.6-80.7 176.7-140.2 209.4-54.8 29.8-90.3 50-90.3 123l-98 0c0-131.6 80.7-176.8 140.2-209.4z";
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
    exports.faHotjar = exports.definition;
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

// src/_entries/faHotjar.js
var import_faHotjar = __toESM(require_faHotjar());
var export_faHotjar = import_faHotjar.definition;
export {
  export_faHotjar as faHotjar
};
