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

// src/node_modules/@fortawesome/free-brands-svg-icons/faTencentWeibo.js
var require_faTencentWeibo = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faTencentWeibo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "tencent-weibo";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "f1d5";
    var svgPathData = "M40.5 495.8c1.4 19.9-27.6 22.2-29.7 2.9-11.6-129.9 31.1-239.5 101.4-313.2-15.6-34 9.2-77.1 50.6-77.1 30.3 0 55.1 24.6 55.1 55.1 0 44-49.5 70.8-86.9 45.1-65.7 71.3-101.4 169.8-90.5 287.2zM160.2 .1c-125.9 0-204.3 134.2-148.3 242.3 8.7 17.4 35.3 4.5 26.3-13.4-46.3-92.6 21-199.2 122-199.2 75.4 0 136.9 61.4 136.9 136.9 0 90.8-86.9 153.9-167.7 133.1-19.1-4.1-25.6 24.4-6.6 29.1 110.7 23.2 204-60 204-162.3 0-91.9-74.6-166.5-166.6-166.5z";
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
    exports.faTencentWeibo = exports.definition;
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

// src/_entries/faTencentWeibo.js
var import_faTencentWeibo = __toESM(require_faTencentWeibo());
var export_faTencentWeibo = import_faTencentWeibo.definition;
export {
  export_faTencentWeibo as faTencentWeibo
};
